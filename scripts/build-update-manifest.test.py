#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("build-update-manifest.py")
SPEC = importlib.util.spec_from_file_location("build_update_manifest", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class UpdateManifestTests(unittest.TestCase):
    def create_assets(self, directory: Path, version: str = "0.2.2") -> None:
        names = [
            f"RADsuite_{version}_Apple-Silicon.dmg",
            f"RADsuite_{version}_Apple-Silicon.app.tar.gz",
            f"RADsuite_{version}_Apple-Silicon.app.tar.gz.sig",
            f"RADsuite_{version}_Intel.dmg",
            f"RADsuite_{version}_Intel.app.tar.gz",
            f"RADsuite_{version}_Intel.app.tar.gz.sig",
            f"RADsuite_{version}_Windows_x64_Setup.exe",
            f"RADsuite_{version}_Windows_x64_Setup.exe.sig",
            f"RADsuite_{version}_Windows_x64.msi",
            f"RADsuite_{version}_Windows_x64.msi.sig",
        ]
        for name in names:
            (directory / name).write_text("signature" if name.endswith(".sig") else "binary", encoding="ascii")

    def test_builds_platform_manifest_and_checksums(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_assets(directory)
            manifest = directory / "latest.json"
            checksums = directory / "SHA256SUMS.txt"
            result = MODULE.build_manifest(directory, "0.2.2", "radicalmove/radsuite", "2026-08-12T00:00:00Z")
            manifest.write_text(json.dumps(result), encoding="utf-8")
            MODULE.write_checksums(directory, checksums, "0.2.2")
            self.assertEqual(set(result["platforms"]), {"darwin-aarch64", "darwin-x86_64", "windows-x86_64"})
            self.assertIn("releases/download/v0.2.2/RADsuite_0.2.2_Windows_x64_Setup.exe", result["platforms"]["windows-x86_64"]["url"])
            self.assertNotIn(".msi", result["platforms"]["windows-x86_64"]["url"])
            self.assertEqual(len(checksums.read_text(encoding="ascii").splitlines()), 10)

    def test_rejects_prereleases_and_missing_signatures(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_assets(directory)
            (directory / "RADsuite_0.2.2_Apple-Silicon.app.tar.gz.sig").unlink()
            with self.assertRaisesRegex(ValueError, "missing release asset"):
                MODULE.build_manifest(directory, "0.2.2", "radicalmove/radsuite", "2026-08-12T00:00:00Z")
            with self.assertRaisesRegex(ValueError, "without a prerelease"):
                MODULE.build_manifest(directory, "0.2.2-rc.1", "radicalmove/radsuite", "2026-08-12T00:00:00Z")


if __name__ == "__main__":
    unittest.main()
