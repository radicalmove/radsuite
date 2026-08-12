#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
import importlib.util


PATH = Path(__file__).with_name("validate-update-manifest.py")
SPEC = importlib.util.spec_from_file_location("validate_update_manifest", PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class UpdateManifestValidationTests(unittest.TestCase):
    def test_accepts_generated_manifest_and_assets(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            version = "0.2.2"
            platforms = {
                "darwin-aarch64": "Apple-Silicon.app.tar.gz",
                "darwin-x86_64": "Intel.app.tar.gz",
                "windows-x86_64": "Windows_x64_Setup.exe",
            }
            entries = {}
            for platform, suffix in platforms.items():
                name = f"RADsuite_{version}_{suffix}"
                (directory / name).write_text("binary", encoding="ascii")
                (directory / f"{name}.sig").write_text("signature", encoding="ascii")
                entries[platform] = {
                    "signature": "signature",
                    "url": f"https://github.com/radicalmove/radsuite/releases/download/v{version}/{name}",
                }
            manifest = directory / "latest.json"
            manifest.write_text(json.dumps({"version": version, "platforms": entries}), encoding="utf-8")
            MODULE.validate(manifest, directory, version, "radicalmove/radsuite")

    def test_rejects_non_https_or_msi_windows_target(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            manifest = directory / "latest.json"
            manifest.write_text(json.dumps({"version": "0.2.2", "platforms": {}}), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "platform keys"):
                MODULE.validate(manifest, directory, "0.2.2", "radicalmove/radsuite")

    def test_rejects_a_signature_that_does_not_match_the_asset_sidecar(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            entries = {}
            for platform, suffix in MODULE.PLATFORMS.items():
                name = f"RADsuite_0.2.2_{suffix}"
                (directory / name).write_text("binary", encoding="ascii")
                (directory / f"{name}.sig").write_text("different", encoding="ascii")
                entries[platform] = {
                    "signature": "signature",
                    "url": f"https://github.com/radicalmove/radsuite/releases/download/v0.2.2/{name}",
                }
            manifest = directory / "latest.json"
            manifest.write_text(json.dumps({"version": "0.2.2", "platforms": entries}), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "does not match"):
                MODULE.validate(manifest, directory, "0.2.2", "radicalmove/radsuite")


if __name__ == "__main__":
    unittest.main()
