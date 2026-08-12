#!/usr/bin/env python3
"""Build the static Tauri updater manifest and release checksums."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote


VERSION_PATTERN = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--asset-dir", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--checksums", type=Path, required=True)
    parser.add_argument("--pub-date", default=None)
    return parser.parse_args()


def require_stable_version(version: str) -> None:
    if not VERSION_PATTERN.fullmatch(version):
        raise ValueError(f"stable release version must be SemVer without a prerelease suffix: {version}")


def require_file(asset_dir: Path, filename: str) -> Path:
    path = asset_dir / filename
    if not path.is_file():
        raise ValueError(f"missing release asset: {filename}")
    return path


def read_signature(asset_dir: Path, filename: str) -> str:
    signature = require_file(asset_dir, filename).read_text(encoding="utf-8").strip()
    if not signature:
        raise ValueError(f"empty updater signature: {filename}")
    return signature


def build_manifest(asset_dir: Path, version: str, repository: str, pub_date: str) -> dict:
    require_stable_version(version)
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError(f"invalid GitHub repository: {repository}")
    owner, name = repository.split("/", 1)
    base_url = f"https://github.com/{owner}/{name}/releases/download/v{version}"

    updater_assets = {
        "darwin-aarch64": f"RADsuite_{version}_Apple-Silicon.app.tar.gz",
        "darwin-x86_64": f"RADsuite_{version}_Intel.app.tar.gz",
        "windows-x86_64": f"RADsuite_{version}_Windows_x64_Setup.exe",
    }
    platforms = {}
    for platform, asset_name in updater_assets.items():
        signature = read_signature(asset_dir, f"{asset_name}.sig")
        require_file(asset_dir, asset_name)
        platforms[platform] = {
            "signature": signature,
            "url": f"{base_url}/{quote(asset_name)}",
        }

    for asset_name in (
        f"RADsuite_{version}_Apple-Silicon.dmg",
        f"RADsuite_{version}_Intel.dmg",
        f"RADsuite_{version}_Windows_x64.msi",
    ):
        require_file(asset_dir, asset_name)
    require_file(asset_dir, f"RADsuite_{version}_Windows_x64.msi.sig")

    return {
        "version": version,
        "notes": f"RADsuite {version} stable update.",
        "pub_date": pub_date,
        "platforms": platforms,
    }


def write_checksums(asset_dir: Path, output: Path, version: str) -> None:
    prefix = f"RADsuite_{version}_"
    assets = sorted(
        path
        for path in asset_dir.iterdir()
        if path.is_file() and path.name.startswith(prefix) and not path.name.endswith((".json", ".txt"))
    )
    if not assets:
        raise ValueError("no release binary or signature assets found")
    lines = []
    for path in assets:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        lines.append(f"{digest}  {path.name}")
    output.write_text("\n".join(lines) + "\n", encoding="ascii")


def main() -> int:
    args = parse_args()
    pub_date = args.pub_date or datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    manifest = build_manifest(args.asset_dir, args.version, args.repository, pub_date)
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    write_checksums(args.asset_dir, args.checksums, args.version)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ValueError as error:
        raise SystemExit(f"error: {error}")
