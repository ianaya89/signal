#!/usr/bin/env bash
# Writes <version> into every manifest that carries one. The git tag is the
# single source of truth for a release, so both release-local.sh and the CI
# workflow call this before building instead of relying on a manual pre-tag bump.
#
#   ./scripts/set-version.sh 0.2.0
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${1:-}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]] || {
  echo "usage: ./scripts/set-version.sh <semver>   e.g. ./scripts/set-version.sh 0.2.0" >&2
  exit 1
}

python3 - "$VERSION" <<'PY'
import json
import re
import sys

version = sys.argv[1]

# Cargo workspace: only the [workspace.package] version, never a dependency's.
cargo = open("Cargo.toml", encoding="utf-8").read()
patched, n = re.subn(
    r'(\[workspace\.package\][^\[]*?\bversion\s*=\s*")[^"]+(")',
    lambda m: m.group(1) + version + m.group(2),
    cargo,
    count=1,
    flags=re.S,
)
if n != 1:
    sys.exit("error: [workspace.package] version not found in Cargo.toml")
open("Cargo.toml", "w", encoding="utf-8").write(patched)

for path, key in (("src-tauri/tauri.conf.json", "version"), ("package.json", "version")):
    with open(path, encoding="utf-8") as fh:
        data = json.load(fh)
    data[key] = version
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(data, fh, indent=2, ensure_ascii=False)
        fh.write("\n")

print(f"version set to {version} in Cargo.toml, src-tauri/tauri.conf.json, package.json")
PY
