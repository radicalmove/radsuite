#!/usr/bin/env python3

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import importlib.util


def load(name: str) -> object:
    path = Path(__file__).with_name(f"{name}.py")
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


MODULE = load("validate-release-version")


class ReleaseVersionTests(unittest.TestCase):
    def test_accepts_matching_application_manifests(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "apps/desktop-ui/src-tauri").mkdir(parents=True)
            (root / "crates/radsuite-desktop").mkdir(parents=True)
            for path in (
                root / "apps/desktop-ui/src-tauri/Cargo.toml",
                root / "crates/radsuite-desktop/Cargo.toml",
            ):
                path.write_text("[package]\nversion = \"0.2.2\"\n", encoding="utf-8")
            (root / "apps/desktop-ui/package.json").write_text('{"version":"0.2.2"}', encoding="utf-8")
            (root / "apps/desktop-ui/package-lock.json").write_text('{"packages":{"":{"version":"0.2.2"}}}', encoding="utf-8")
            (root / "apps/desktop-ui/src-tauri/tauri.conf.json").write_text('{"version":"0.2.2"}', encoding="utf-8")
            self.assertEqual(MODULE.validate(root, "v0.2.2"), "0.2.2")

    def test_rejects_mismatched_tag_or_manifest(self) -> None:
        with self.assertRaisesRegex(ValueError, "release tag"):
            MODULE.validate(Path("/tmp"), "0.2.2")


if __name__ == "__main__":
    unittest.main()
