#!/bin/bash
# Publish every workspace crate in dependency order. crates.io allows a burst
# of publishes followed by roughly one per minute, so pace the uploads and
# retry when the rate limit is hit. Versions already on crates.io are skipped
# so an interrupted run can resume without republishing.
set -euo pipefail

crates=$(python3 - <<'PY'
import json
import subprocess

metadata = json.loads(
    subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"])
)
workspace = {package["name"] for package in metadata["packages"]}
versions = {package["name"]: package["version"] for package in metadata["packages"]}
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

print("\n".join(f"{name}\t{versions[name]}" for name in ordered))
PY
)

published() {
  local crate=$1 version=$2 status
  status=$(curl -sS --max-time 30 -o /dev/null -w '%{http_code}' \
    -A 'llrt-publish (https://github.com/awslabs/llrt)' \
    "https://crates.io/api/v1/crates/${crate}/${version}") || return 1
  [ "$status" = 200 ]
}

while IFS=$'\t' read -r crate version; do
  if published "$crate" "$version"; then
    echo "Skipping $crate $version (already published)"
    continue
  fi

  attempt=1
  while true; do
    if output=$(cargo publish -p "$crate" 2>&1); then
      echo "$output"
      sleep 65
      break
    fi

    echo "$output" >&2
    if grep -qiE "already (uploaded|exists|published)" <<<"$output"; then
      echo "Skipping $crate $version (already published)"
      break
    fi
    if ! grep -q "status 429" <<<"$output" || [ "$attempt" -ge 10 ]; then
      exit 1
    fi
    attempt=$((attempt + 1))
    echo "Rate limited, retrying $crate in 65 seconds (attempt $attempt)" >&2
    sleep 65
  done
done <<<"$crates"
