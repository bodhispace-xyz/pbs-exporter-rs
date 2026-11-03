"""Validate release identity before publishing images or binary assets."""

import argparse
import json
import os
from pathlib import Path
import re
import tomllib


def release_context(root: Path, tag: str) -> dict[str, str]:
    cargo = tomllib.loads((root / "Cargo.toml").read_text())
    version = cargo["package"]["version"]
    manifest = json.loads((root / ".release-please-manifest.json").read_text())
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    locked = next(p["version"] for p in lock["package"] if p["name"] == cargo["package"]["name"])
    if version != manifest["."] or version != locked:
        raise ValueError("Cargo.toml, Cargo.lock and the release manifest must agree")
    if tag:
        if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", tag):
            raise ValueError("Release tags must use v<semantic-version>")
        if tag != f"v{version}":
            raise ValueError(f"Release tag {tag} does not match package version {version}")
    return {"is_release": "true" if tag else "false", "version": version, "tag": tag}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", default="")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    try:
        context = release_context(args.root, args.tag)
    except (ValueError, KeyError, StopIteration) as error:
        parser.exit(1, f"Invalid release context: {error}\n")
    if output := os.environ.get("GITHUB_OUTPUT"):
        with open(output, "a") as stream:
            for key, value in context.items():
                stream.write(f"{key}={value}\n")
    print(json.dumps(context))


if __name__ == "__main__":
    main()
