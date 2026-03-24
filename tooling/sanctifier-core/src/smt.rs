//! Z3-based formal-verification primitives.

use serde::Serialize;
use std::time::Instant;
use z3::ast::Int;
use z3::{Context, SatResult, Solver};

/// An invariant issue proved by the Z3 SMT solver.
#[derive(Debug, Serialize, Clone)]
pub struct SmtInvariantIssue {
    /// Function under verification.
    pub function_name: String,
    /// Human-readable description of the violation.
    pub description: String,
    /// Source location.
    pub location: String,
}

/// Z3-backed SMT solver wrapper.
pub struct SmtVerifier<'ctx> {
    ctx: &'ctx Context,
}

impl<'ctx> SmtVerifier<'ctx> {
    /// Create a verifier bound to a Z3 [`Context`].
    pub fn new(ctx: &'ctx Context) -> Self {
        Self { ctx }
    }

    /// Proof-of-Concept: Uses Z3 to prove if `a + b` can overflow a 64-bit integer
    /// under unconstrained conditions.
    pub fn verify_addition_overflow(
        &self,
        fn_name: &str,
        location: &str,
    ) -> Option<SmtInvariantIssue> {
        let solver = Solver::new(self.ctx);
        let a = Int::new_const(self.ctx, "a");
        let b = Int::new_const(self.ctx, "b");

        // u64 bounds
        let zero = Int::from_u64(self.ctx, 0);
        let max_u64 = Int::from_u64(self.ctx, u64::MAX);

        // Constrain variables to valid u64 limits: 0 <= a, b <= u64::MAX
        solver.assert(&a.ge(&zero));
        solver.assert(&a.le(&max_u64));
        solver.assert(&b.ge(&zero));
        solver.assert(&b.le(&max_u64));

        // To prove overflow is IMPOSSIBLE, we assert the violation (a + b > max_u64)
        // and check if the solver can SATISFY this violation.
        let sum = Int::add(self.ctx, &[&a, &b]);
        solver.assert(&sum.gt(&max_u64));

        if solver.check() == SatResult::Sat {
            // A model exists where a + b > u64::MAX, meaning an overflow is mathematically possible
            Some(SmtInvariantIssue {
                function_name: fn_name.to_string(),
                description: "SMT Solver (Z3) proved that this addition can overflow u64 bounds."
                    .to_string(),
                location: location.to_string(),
            })
        } else {
            None
        }
    }
}

/// The constraint-generation strategy used for an SMT proof.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SmtProofStrategy {
    /// Full u64 range.
    UnconstrainedOverflow,
    /// Bounded to ~5 × 10⁹.
    BoundedDomainOverflow,
    /// Bounded to 10 000.
    SmallDomainOverflow,
}

/// Latency statistics for a single [`SmtProofStrategy`].
#[derive(Debug, Serialize, Clone)]
pub struct SmtStrategyLatency {
    /// Which strategy was measured.
    pub strategy: SmtProofStrategy,
    /// Number of iterations.
    pub runs: usize,
    /// Fastest run in microseconds.
    pub min_micros: u128,
    /// Slowest run in microseconds.
    pub max_micros: u128,
    /// Mean duration in microseconds.
    pub avg_micros: u128,
    /// 95th-percentile duration in microseconds.
    pub p95_micros: u128,
}

/// Aggregate benchmark across all [`SmtProofStrategy`] variants.
#[derive(Debug, Serialize, Clone)]
pub struct SmtLatencyBenchmarkReport {
    /// How many iterations were run per strategy.
    pub iterations_per_strategy: usize,
    /// Per-strategy results.
    pub strategies: Vec<SmtStrategyLatency>,
}

impl SmtLatencyBenchmarkReport {
    /// Return strategies ordered by descending average latency.
    pub fn most_expensive_first(&self) -> Vec<SmtStrategyLatency> {
        let mut sorted = self.strategies.clone();
        sorted.sort_by(|a, b| b.avg_micros.cmp(&a.avg_micros));
        sorted
    }
}

/// Run a latency micro-benchmark for each [`SmtProofStrategy`].
pub fn run_smt_latency_benchmark(iterations_per_strategy: usize) -> SmtLatencyBenchmarkReport {
    use z3::{Config, Context};

    let iterations = iterations_per_strategy.max(1);
    let strategies = [
        SmtProofStrategy::UnconstrainedOverflow,
        SmtProofStrategy::BoundedDomainOverflow,
        SmtProofStrategy::SmallDomainOverflow,
    ];

    let mut results = Vec::with_capacity(strategies.len());

    for strategy in strategies {
        let mut samples = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let cfg = Config::new();
            let ctx = Context::new(&cfg);

            let start = Instant::now();
            let _ = run_strategy(&ctx, strategy);
            samples.push(start.elapsed().as_micros());
        }

        samples.sort_unstable();
        let min_micros = samples.first().copied().unwrap_or_default();
        let max_micros = samples.last().copied().unwrap_or_default();
        let total: u128 = samples.iter().sum();
        let avg_micros = total / samples.len() as u128;
        let p95_index = (((samples.len() - 1) as f64) * 0.95).round() as usize;
        let p95_micros = samples[p95_index];

        results.push(SmtStrategyLatency {
            strategy,
            runs: iterations,
            min_micros,
            max_micros,
            avg_micros,
            p95_micros,
        });
    }

    SmtLatencyBenchmarkReport {
        iterations_per_strategy: iterations,
        strategies: results,
    }
}

fn run_strategy(ctx: &Context, strategy: SmtProofStrategy) -> SatResult {
    let solver = Solver::new(ctx);
    let a = Int::new_const(ctx, "a");
    let b = Int::new_const(ctx, "b");
    let zero = Int::from_i64(ctx, 0);
    let max_u64 = Int::from_u64(ctx, u64::MAX);

    solver.assert(&a.ge(&zero));
    solver.assert(&b.ge(&zero));

    match strategy {
        SmtProofStrategy::UnconstrainedOverflow => {
            solver.assert(&a.le(&max_u64));
            solver.assert(&b.le(&max_u64));
        }
        SmtProofStrategy::BoundedDomainOverflow => {
            let max = Int::from_i64(ctx, 5_000_000_000);
            solver.assert(&a.le(&max));
            solver.assert(&b.le(&max));
        }
        SmtProofStrategy::SmallDomainOverflow => {
            let max = Int::from_i64(ctx, 10_000);
            solver.assert(&a.le(&max));
            solver.assert(&b.le(&max));
        }
    }

    let sum = Int::add(ctx, &[&a, &b]);
    solver.assert(&sum.gt(&max_u64));
    solver.check()
}
