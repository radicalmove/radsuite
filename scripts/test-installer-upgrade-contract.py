#!/usr/bin/env python3

"""Regression checks for the stable installer and public download contract."""

from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CONFIG_PATH = ROOT / "apps/desktop-ui/src-tauri/tauri.conf.json"
DOWNLOAD_PAGE = ROOT / "site/download/index.html"


class InstallerUpgradeContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.config = json.loads(CONFIG_PATH.read_text(encoding="utf-8"))
        cls.page = DOWNLOAD_PAGE.read_text(encoding="utf-8").lower()

    def test_stable_identity_and_per_user_windows_install_are_explicit(self) -> None:
        self.assertEqual(self.config["identifier"], "nz.radsuite.app")
        self.assertEqual(
            self.config["bundle"]["windows"]["nsis"]["installMode"],
            "currentUser",
        )

    def test_public_page_explains_upgrade_without_uninstall(self) -> None:
        self.assertIn("do not normally need to uninstall", self.page)
        self.assertIn("existing installation", self.page)

    def test_public_page_uses_stable_download_aliases(self) -> None:
        for asset in (
            "radsuite_apple-silicon.dmg",
            "radsuite_intel.dmg",
            "radsuite_windows_x64_setup.exe",
        ):
            self.assertIn(f"latest/download/{asset}", self.page)


if __name__ == "__main__":
    unittest.main()
