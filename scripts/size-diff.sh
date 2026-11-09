#!/usr/bin/env python3
import json
import sys
from pathlib import Path

if len(sys.argv) < 3 or not Path(sys.argv[1]).exists():
    print("Size baseline unavailable.")
    sys.exit(0)

with open(sys.argv[1]) as f:
    base_data = json.load(f)
with open(sys.argv[2]) as f:
    cur_data = json.load(f)

rows = [
    ("llrt (linux)", "x86_64"),
    ("llrt (lambda compressed)", "x86_64"),
    ("llrt (lambda)", "x86_64"),
    ("llrt (linux)", "aarch64"),
    ("llrt (lambda compressed)", "aarch64"),
    ("llrt (lambda)", "aarch64"),
]

print("### Binary size\n")
print("| Binary | Arch | Base | PR | Δ | Δ % |")
print("|---|---:|---:|---:|---:|---:|")

for name, arch in rows:
    key = f"{name} {arch}"
    b, c = base_data.get(key), cur_data.get(key)

    if b is None or c is None:
        print(f"| `{name}` | {arch} | — | — | — | — |")
        continue

    d = c - b
    delta = f"{d:,}" if d <= 0 else f"+{d:,}"
    pct = f"{(d * 100 / b):+.2f}%" if b else "0.00%"

    if d <= -1024:
        icon = "🟢"
    elif d < 1024:
        icon = "⚪"
    elif d >= 32768 or d * 1000 >= b * 5:
        icon = "🔴"
    else:
        icon = "🟡"

    print(f"| `{name}` | {arch} | {b:,} | {c:,} | {icon} {delta} | {pct} |")
