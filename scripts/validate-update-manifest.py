#!/usr/bin/env python3
"""Validate a static Tauri updater manifest and its local release assets."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from urllib.parse import urlparse


PLATFORMS = {
    "darwin-aarch64": "Apple-Silicon.app.tar.gz",
    "darwin-x86_64": "Intel.app.tar.gz",
    "windows-x86_64": "Windows_x64_Setup.exe",
}


def validate(manifest_path: Path, asset_dir: Path, version: str, repository: str) -> None:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("version") != version:
        raise ValueError(f"manifest version is {manifest.get('version')}, expected {version}")
    if set(manifest.get("platforms", {})) != set(PLATFORMS):
        raise ValueError("manifest platform keys do not match the supported targets")
    base_url = f"https://github.com/{repository}/releases/download/v{version}/"
    for platform, suffix in PLATFORMS.items():
        entry = manifest["platforms"][platform]
        signature = entry.get("signature")
        url = entry.get("url")
        if not isinstance(signature, str) or not signature.strip():
            raise ValueError(f"missing signature for {platform}")
        if not isinstance(url, str) or not url.startswith(base_url):
            raise ValueError(f"invalid release URL for {platform}")
        parsed = urlparse(url)
        if parsed.scheme != "https" or parsed.netloc != "github.com":
            raise ValueError(f"release URL is not HTTPS GitHub for {platform}")
        asset_name = Path(parsed.path).name
        if not asset_name.endswith(suffix):
            raise ValueError(f"unexpected updater asset for {platform}: {asset_name}")
        if not (asset_dir / asset_name).is_file():
            raise ValueError(f"missing updater asset: {asset_name}")
        signature_path = asset_dir / f"{asset_name}.sig"
        if not signature_path.is_file():
            raise ValueError(f"missing updater signature asset: {asset_name}.sig")
        if signature_path.read_text(encoding="utf-8").strip() != signature.strip():
            raise ValueError(f"manifest signature does not match {asset_name}.sig")
    if ".msi" in manifest["platforms"]["windows-x86_64"]["url"].lower():
        raise ValueError("Windows automatic updater must use the NSIS setup executable")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--asset-dir", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--repository", required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", args.repository):
        raise ValueError(f"invalid repository: {args.repository}")
    validate(args.manifest, args.asset_dir, args.version, args.repository)
    print("update manifest is valid")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, json.JSONDecodeError, KeyError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
