import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]


class ActionInputTests(unittest.TestCase):
    def test_accepts_normalized_valid_inputs(self) -> None:
        from scripts.action_inputs import validate_inputs

        got = validate_inputs(
            path=".",
            min_severity="HIGH",
            format="SARIF",
            upload_sarif="yes",
            sarif_output="reports/results.sarif",
            debug="TRUE",
        )

        self.assertEqual(got.path, ".")
        self.assertEqual(got.min_severity, "high")
        self.assertEqual(got.format, "sarif")
        self.assertEqual(got.upload_sarif, "true")
        self.assertEqual(got.sarif_output, "reports/results.sarif")
        self.assertEqual(got.debug, "true")

    def test_rejects_unknown_format(self) -> None:
        from scripts.action_inputs import validate_inputs

        with self.assertRaisesRegex(ValueError, "format must be one of"):
            validate_inputs(
                path=".",
                min_severity="high",
                format="xml",
                upload_sarif="true",
                sarif_output="out.sarif",
                debug="false",
            )

    def test_rejects_path_traversal(self) -> None:
        from scripts.action_inputs import validate_inputs

        with self.assertRaisesRegex(ValueError, "path traversal"):
            validate_inputs(
                path="../outside",
                min_severity="high",
                format="sarif",
                upload_sarif="true",
                sarif_output="out.sarif",
                debug="false",
            )

    def test_rejects_missing_scan_path(self) -> None:
        from scripts.action_inputs import validate_inputs

        with self.assertRaisesRegex(ValueError, "does not exist"):
            validate_inputs(
                path="missing-contract-dir",
                min_severity="high",
                format="sarif",
                upload_sarif="true",
                sarif_output="out.sarif",
                debug="false",
            )


if __name__ == "__main__":
    unittest.main()
