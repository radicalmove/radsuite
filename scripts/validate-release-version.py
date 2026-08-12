#!/usr/bin/env python3
"""Validate that a release tag matches every RADsuite application manifest."""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path


SEMVER = re.compile(r"^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")


def read_toml_version(path: Path) -> str:
    with path.open("rb") as handle:
        return str(tomllib.load(handle)["package"]["version"])


def validate(root: Path, tag: str) -> str:
    match = SEMVER.fullmatch(tag)
    if not match:
        raise ValueError(f"release tag must be v<major>.<minor>.<patch>: {tag}")
    version = tag[1:]
    paths = {
        "desktop Cargo manifest": root / "apps/desktop-ui/src-tauri/Cargo.toml",
        "desktop core Cargo manifest": root / "crates/radsuite-desktop/Cargo.toml",
    }
    versions = {label: read_toml_version(path) for label, path in paths.items()}
    package = json.loads((root / "apps/desktop-ui/package.json").read_text(encoding="utf-8"))
    lock = json.loads((root / "apps/desktop-ui/package-lock.json").read_text(encoding="utf-8"))
    versions["frontend package"] = str(package["version"])
    versions["frontend lockfile"] = str(lock["packages"][""]["version"])
    config = json.loads((root / "apps/desktop-ui/src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
    versions["Tauri config"] = str(config["version"])
    for label, candidate in versions.items():
        if candidate != version:
            raise ValueError(f"{label} is {candidate}, expected {version}")
    return version


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--tag", required=True)
    args = parser.parse_args()
    print(validate(args.root, args.tag))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, KeyError, json.JSONDecodeError, tomllib.TOMLDecodeError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
