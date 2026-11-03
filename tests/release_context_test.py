"""Exercise the publisher's actual version/tag validation with isolated fixtures."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "release_context", Path(__file__).resolve().parents[1] / "scripts/release_context.py"
)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseContextTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "Cargo.toml").write_text('[package]\nname="pbs-exporter"\nversion="0.4.6"\n')
        (self.root / "Cargo.lock").write_text('[[package]]\nname="pbs-exporter"\nversion="0.4.6"\n')
        (self.root / ".release-please-manifest.json").write_text(json.dumps({".": "0.4.6"}))

    def test_main_build_is_not_a_release(self):
        self.assertEqual(module.release_context(self.root, ""), {
            "is_release": "false", "version": "0.4.6", "tag": ""
        })

    def test_matching_release_is_accepted(self):
        self.assertEqual(module.release_context(self.root, "v0.4.6"), {
            "is_release": "true", "version": "0.4.6", "tag": "v0.4.6"
        })

    def test_wrong_version_or_tag_format_is_rejected(self):
        for tag in ["v0.4.7", "pbs-exporter-rs-v0.4.6", "main", "v0.4.6\nextra=true", "v0.4.6;echo bad"]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                module.release_context(self.root, tag)

    def test_manifest_drift_is_rejected(self):
        (self.root / ".release-please-manifest.json").write_text(json.dumps({".": "0.4.7"}))
        with self.assertRaisesRegex(ValueError, "must agree"):
            module.release_context(self.root, "v0.4.6")

    def test_lockfile_drift_is_rejected(self):
        (self.root / "Cargo.lock").write_text('[[package]]\nname="pbs-exporter"\nversion="0.4.5"\n')
        with self.assertRaisesRegex(ValueError, "must agree"):
            module.release_context(self.root, "v0.4.6")


if __name__ == "__main__":
    unittest.main()
