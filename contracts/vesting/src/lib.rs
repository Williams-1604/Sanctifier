#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    Schedule,
}

// ── Data types ────────────────────────────────────────────────────────────────

/// A linear vesting schedule with an optional cliff and admin-revocable unvested tokens.
#[contracttype]
#[derive(Clone)]
pub struct VestingSchedule {
    /// Address that will receive the vested tokens.
    pub beneficiary: Address,
    /// Token contract address (SEP-41 compliant).
    pub token: Address,
    /// Total tokens locked into this schedule.
    pub total_amount: i128,
    /// Tokens already claimed by the beneficiary.
    pub claimed_amount: i128,
    /// Unix timestamp (seconds) at which vesting begins.
    pub start_time: u64,
    /// Seconds after `start_time` before any tokens vest (cliff).
    /// During [start_time, start_time + cliff_duration) nothing is claimable.
    pub cliff_duration: u64,
    /// Total seconds over which all tokens become vested (must be >= cliff_duration).
    pub vesting_duration: u64,
    /// True once the admin has called `revoke`.
    pub revoked: bool,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct VestingContract;

#[contractimpl]
impl VestingContract {
    /// Create a new vesting schedule.
    ///
    /// The caller (`admin`) must have approved this contract to transfer
    /// `total_amount` tokens on their behalf before calling `initialize`.
    ///
    /// # Arguments
    /// * `admin`            – Account that funds and may later revoke the schedule.
    /// * `token`            – SEP-41 token contract address.
    /// * `beneficiary`      – Recipient of vested tokens.
    /// * `total_amount`     – Tokens to lock (must be > 0).
    /// * `start_time`       – Unix timestamp vesting starts.
    /// * `cliff_duration`   – Seconds before first token vests (0 = no cliff).
    /// * `vesting_duration` – Total vesting window in seconds (must be > 0 and >= cliff_duration).
    pub fn initialize(
        env: Env,
        admin: Address,
        token: Address,
        beneficiary: Address,
        total_amount: i128,
        start_time: u64,
        cliff_duration: u64,
        vesting_duration: u64,
    ) {
        assert!(
            !env.storage().instance().has(&DataKey::Admin),
            "already initialized"
        );
        assert!(total_amount > 0, "total_amount must be positive");
        assert!(vesting_duration > 0, "vesting_duration must be positive");
        assert!(
            cliff_duration <= vesting_duration,
            "cliff_duration must not exceed vesting_duration"
        );

        admin.require_auth();

        token::Client::new(&env, &token).transfer(
            &admin,
            &env.current_contract_address(),
            &total_amount,
        );

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(
            &DataKey::Schedule,
            &VestingSchedule {
                beneficiary,
                token,
                total_amount,
                claimed_amount: 0,
                start_time,
                cliff_duration,
                vesting_duration,
                revoked: false,
            },
        );
    }

    /// Returns the number of tokens the beneficiary can claim right now.
    pub fn claimable(env: Env) -> i128 {
        let schedule: VestingSchedule =
            env.storage().instance().get(&DataKey::Schedule).unwrap();
        if schedule.revoked {
            return 0;
        }
        Self::vested_amount(&env, &schedule) - schedule.claimed_amount
    }

    /// Transfer all currently vested (and unclaimed) tokens to the beneficiary.
    ///
    /// Only the beneficiary may call this.
    pub fn claim(env: Env) -> i128 {
        let mut schedule: VestingSchedule =
            env.storage().instance().get(&DataKey::Schedule).unwrap();
        schedule.beneficiary.require_auth();
        assert!(!schedule.revoked, "schedule has been revoked");

        let vested = Self::vested_amount(&env, &schedule);
        let claimable = vested - schedule.claimed_amount;
        assert!(claimable > 0, "nothing to claim");

        schedule.claimed_amount = vested;
        env.storage().instance().set(&DataKey::Schedule, &schedule);

        token::Client::new(&env, &schedule.token).transfer(
            &env.current_contract_address(),
            &schedule.beneficiary,
            &claimable,
        );

        claimable
    }

    /// Revoke the vesting schedule.
    ///
    /// Any tokens that have vested but not yet been claimed are immediately
    /// transferred to the beneficiary.  All remaining unvested tokens are
    /// returned to the admin.  Only the admin may call this.
    pub fn revoke(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut schedule: VestingSchedule =
            env.storage().instance().get(&DataKey::Schedule).unwrap();
        assert!(!schedule.revoked, "schedule already revoked");

        let vested = Self::vested_amount(&env, &schedule);
        let beneficiary_due = vested - schedule.claimed_amount;
        if beneficiary_due > 0 {
            token::Client::new(&env, &schedule.token).transfer(
                &env.current_contract_address(),
                &schedule.beneficiary,
                &beneficiary_due,
            );
        }

        let unvested = schedule.total_amount - vested;
        if unvested > 0 {
            token::Client::new(&env, &schedule.token).transfer(
                &env.current_contract_address(),
                &admin,
                &unvested,
            );
        }

        schedule.revoked = true;
        schedule.claimed_amount = vested;
        env.storage().instance().set(&DataKey::Schedule, &schedule);
    }

    /// Returns the full vesting schedule (read-only).
    pub fn schedule(env: Env) -> VestingSchedule {
        env.storage().instance().get(&DataKey::Schedule).unwrap()
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn vested_amount(env: &Env, s: &VestingSchedule) -> i128 {
        let now = env.ledger().timestamp();
        if now < s.start_time + s.cliff_duration {
            // Before cliff — nothing has vested yet.
            return 0;
        }
        let elapsed = now.saturating_sub(s.start_time);
        if elapsed >= s.vesting_duration {
            // Past the end of the vesting window — everything is vested.
            s.total_amount
        } else {
            // Linear interpolation between start and end.
            (s.total_amount * elapsed as i128) / s.vesting_duration as i128
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Env,
    };

    fn setup() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
        (env, admin, beneficiary, token)
    }

    #[test]
    fn test_cliff_blocks_early_claim() {
        let (env, admin, beneficiary, token) = setup();
        let contract_id = env.register(VestingContract, ());
        let client = VestingContractClient::new(&env, &contract_id);

        let now = env.ledger().timestamp();
        client.initialize(&admin, &token, &beneficiary, &1_000_000, &now, &3600, &7200);

        // Before cliff — nothing claimable.
        assert_eq!(client.claimable(), 0);
    }

    #[test]
    fn test_full_vest_after_duration() {
        let (env, admin, beneficiary, token) = setup();
        let contract_id = env.register(VestingContract, ());
        let client = VestingContractClient::new(&env, &contract_id);

        let start = env.ledger().timestamp();
        client.initialize(&admin, &token, &beneficiary, &1_000_000, &start, &0, &3600);

        env.ledger().with_mut(|l| l.timestamp = start + 3600);
        assert_eq!(client.claimable(), 1_000_000);

        let claimed = client.claim();
        assert_eq!(claimed, 1_000_000);
        assert_eq!(client.claimable(), 0);
    }

    #[test]
    fn test_revoke_returns_unvested_to_admin() {
        let (env, admin, beneficiary, token) = setup();
        let contract_id = env.register(VestingContract, ());
        let client = VestingContractClient::new(&env, &contract_id);

        let start = env.ledger().timestamp();
        // No cliff, 7200-second vesting window.
        client.initialize(&admin, &token, &beneficiary, &7200, &start, &0, &7200);

        // Advance to halfway (3600 s) — half of tokens are vested.
        env.ledger().with_mut(|l| l.timestamp = start + 3600);

        client.revoke();

        let schedule = client.schedule();
        assert!(schedule.revoked);
        // Half was vested at revocation; those go to beneficiary.
        // The other half returns to admin.  contract holds nothing.
        assert_eq!(schedule.claimed_amount, 3600_i128);
    }
}
