#!/usr/bin/env bash
set -euo pipefail

base=$1
cur=$2

fmt() {
  local n=$1 s out=
  s=$(printf '%d' "$n")
  local sign=
  if [[ $s == -* ]]; then
    sign=-
    s=${s#-}
  fi
  while (( ${#s} > 3 )); do
    out=",${s: -3}${out}"
    s=${s:0:${#s}-3}
  done
  printf '%s%s%s' "$sign" "$s" "$out"
}

if [[ ! -f $base ]]; then
  echo "Size baseline unavailable."
  exit 0
fi

rows=(
  "llrt (linux)|x86_64"
  "llrt (lambda compressed)|x86_64"
  "llrt (lambda)|x86_64"
  "llrt (linux)|aarch64"
  "llrt (lambda compressed)|aarch64"
  "llrt (lambda)|aarch64"
)

echo "### Binary size"
echo
echo "| Binary | Arch | Base | PR | Δ | Δ % |"
echo "|---|---:|---:|---:|---:|---:|"

for row in "${rows[@]}"; do
  name=${row%|*}
  arch=${row#*|}
  key="$name $arch"
  b=$(jq -r --arg k "$key" '.[$k] // empty' "$base")
  c=$(jq -r --arg k "$key" '.[$k] // empty' "$cur")

  if [[ -z $b || -z $c ]]; then
    printf '| `%s` | %s | — | — | — | — |\n' "$name" "$arch"
    continue
  fi

  d=$((c - b))

  if (( d > 0 )); then
    delta="+$(fmt "$d")"
  elif (( d < 0 )); then
    delta=$(fmt "$d")
  else
    delta=0
  fi

  if (( d == 0 )); then
    pct=0.00%
  else
    pct=$(awk -v d="$d" -v b="$b" 'BEGIN { printf "%+.2f%%", d * 100 / b }')
  fi

  if (( d <= -1024 )); then
    icon=🟢
  elif (( d < 1024 )); then
    icon=⚪
  elif (( d >= 32768 )) || (( d * 1000 >= b * 5 )); then
    icon=🔴
  else
    icon=🟡
  fi

  printf '| `%s` | %s | %s | %s | %s %s | %s |\n' "$name" "$arch" "$(fmt "$b")" "$(fmt "$c")" "$icon" "$delta" "$pct"
done
