#!/bin/bash
# Publish every workspace crate in dependency order. crates.io allows a burst
# of publishes followed by roughly one per minute, so pace the uploads and
# retry when the rate limit is hit.
set -euo pipefail

crates=$(python3 - <<'PY'
import json
import subprocess

metadata = json.loads(
    subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"])
)
workspace = {package["name"] for package in metadata["packages"]}
pending = {
    package["name"]: {
        dependency["name"]
        for dependency in package["dependencies"]
        if dependency["name"] in workspace
    }
    for package in metadata["packages"]
}

ordered = []
while pending:
    ready = sorted(name for name, deps in pending.items() if not deps)
    if not ready:
        raise SystemExit("dependency cycle among workspace crates")
    ordered.extend(ready)
    for name in ready:
        del pending[name]
    for deps in pending.values():
        deps.difference_update(ready)

print("\n".join(ordered))
PY
)

for crate in $crates; do
  attempt=1
  while true; do
    output=$(cargo publish -p "$crate" 2>&1) && break
    echo "$output" >&2
    if ! grep -q "status 429" <<<"$output" || [ "$attempt" -ge 10 ]; then
      exit 1
    fi
    attempt=$((attempt + 1))
    echo "Rate limited, retrying $crate in 65 seconds (attempt $attempt)" >&2
    sleep 65
  done
  echo "$output"
  sleep 65
done
