#!/bin/bash
set -e
crates=$(cargo metadata --no-deps --format-version 1 --quiet | jq -r '.packages[] | select(.manifest_path | contains("modules/")) | .name')
while IFS= read -r crate; do
  echo "Compiling feature: $crate"
  cargo build -p $crate
done <<< "$crates"
