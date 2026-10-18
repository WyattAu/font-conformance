#!/usr/bin/env python3
"""Corpus fetch + attestation for font-conformance (FCN-001).

stdlib only (standards contract: scripts are canonical, no deps). The Rust
side verifies hashes on every run; this script is the only thing that ever
downloads.

    python3 scripts/fetch_corpus.py verify   # check every cached file (default)
    python3 scripts/fetch_corpus.py fetch    # download anything missing, then verify
    python3 scripts/fetch_corpus.py init     # download + print sha256 lines to paste

Cache: $FONT_CONFORMANCE_CACHE or ~/.cache/font-conformance/<name>.
A wrong hash exits non-zero; it is a finding, not a warning.
"""

import hashlib
import os
import pathlib
import sys
import tomllib
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "corpus" / "manifest.toml"


def cache_dir() -> pathlib.Path:
    env = os.environ.get("FONT_CONFORMANCE_CACHE")
    base = pathlib.Path(env) if env else pathlib.Path.home() / ".cache" / "font-conformance"
    base.mkdir(parents=True, exist_ok=True)
    return base


def load_manifest() -> list[dict]:
    with MANIFEST.open("rb") as f:
        data = tomllib.load(f)
    if data.get("schema") != 1:
        sys.exit(f"unsupported manifest schema: {data.get('schema')!r}")
    return data.get("fonts", [])


def sha256_of(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def ensure(entry: dict, fetch_missing: bool) -> pathlib.Path:
    dest = cache_dir() / entry["name"]
    if not dest.exists():
        if not fetch_missing:
            sys.exit(f"missing from cache: {dest} (run `fetch`)")
        req = urllib.request.Request(entry["url"], headers={"User-Agent": "font-conformance/0.1"})
        with urllib.request.urlopen(req, timeout=60) as resp:
            dest.write_bytes(resp.read())
    actual = sha256_of(dest)
    expected = entry.get("sha256", "")
    if not expected:
        print(f'{entry["name"]}: sha256 = "{actual}"')
    elif actual != expected:
        sys.exit(f"HASH MISMATCH for {entry['name']}: expected {expected}, got {actual}")
    return dest


def main() -> int:
    if len(sys.argv) > 2:
        print(__doc__)
        return 2
    mode = sys.argv[1] if len(sys.argv) > 1 else "verify"
    if mode not in {"verify", "fetch", "init"}:
        print(__doc__)
        return 2
    fonts = load_manifest()
    if mode == "init":
        for entry in fonts:
            ensure(entry, fetch_missing=True)
        print(f"\n{len(fonts)} entry/entries hashed; paste the lines above into corpus/manifest.toml")
        return 0
    for entry in fonts:
        path = ensure(entry, fetch_missing=mode == "fetch")
        print(f"ok {entry['name']} ({path.stat().st_size} bytes)")
    print(f"{len(fonts)} entry/entries verified")
    return 0


if __name__ == "__main__":
    sys.exit(main())
