#!/usr/bin/env python3
"""Run the TLS handshake benchmarks and generate a Markdown report."""

import argparse
import datetime
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile

TLS_DIR = Path(__file__).resolve().parent.parent
MANIFEST = TLS_DIR / "handshake-bench" / "Cargo.toml"
PROVIDERS = ("tls-rust", "tls-ring", "tls-graviola", "tls-aws-lc")
CERTIFICATES = (
    "rsa-2048", "rsa-3072", "rsa-4096", "ecdsa-p256",
    "ecdsa-p384", "ecdsa-p521", "ed25519",
)
MODES = ("full", "resumed")


def escape(text):
    return str(text).replace("&", "&amp;").replace("|", "&#124;").replace("\n", " ")


def collect_results(directory, provider, skips):
    results = {}
    for metadata_file in directory.glob("**/new/benchmark.json"):
        metadata = json.loads(metadata_file.read_text())
        if metadata["group_id"] != f"{provider}/tls12":
            raise ValueError(f"Unexpected benchmark ID in {metadata_file}")
        cert, mode = metadata["function_id"], metadata["value_str"]
        key = (cert, mode)
        if cert not in CERTIFICATES or mode not in MODES or key in results:
            raise ValueError(f"Unexpected or duplicate benchmark: {metadata['full_id']}")
        estimates = json.loads((metadata_file.parent / "estimates.json").read_text())
        estimate = estimates.get("slope") or estimates["mean"]
        ns = estimate["point_estimate"]
        if not isinstance(ns, (int, float)) or not math.isfinite(ns) or ns <= 0:
            raise ValueError(f"Invalid estimate for {metadata['full_id']}: {ns}")
        results[key] = ns

    for cert in CERTIFICATES:
        for mode in MODES:
            key = (cert, mode)
            if (key in results) == (key in skips):
                raise ValueError(f"{provider}/{cert}/{mode}: expected a result or skip reason")
    if not results:
        raise ValueError(f"{provider}: no benchmarks produced results")
    return results


def render_report(results, skips, generated, options):
    lines = [
        "# LLRT TLS 1.2 Handshake Benchmark Summary",
        "",
        f"Measured: {generated.strftime('%Y-%m-%d %H:%M:%S UTC')}.",
        f"Environment: {escape(platform.system())} / {escape(platform.machine())}.",
        f"Criterion options: {escape(' '.join(options) if options else 'defaults')}. "
        "Providers were run sequentially.",
        "",
        "In-memory rustls client and server; TLS 1.2, ca-rsa, server authentication. "
        "Full handshakes disable resumption; resumed handshakes use a primed TLS 1.2 "
        "session cache. Each iteration verifies the handshake kind. Results include "
        "client and server work, but exclude runtime startup, TCP and HTTP.",
        "",
        "Each cell shows Criterion's slope estimate (mean when slope is unavailable) "
        "in milliseconds / estimated handshakes per second (rank). Rankings use "
        "unrounded times; failed setup cases are excluded.",
        "",
    ]
    for mode in MODES:
        lines += [
            f"## {mode.title()} Handshakes",
            "",
            "| Provider | " + " | ".join(CERTIFICATES) + " |",
            "| --- | " + " | ".join(["---:"] * len(CERTIFICATES)) + " |",
        ]
        for provider in PROVIDERS:
            cells = []
            for cert in CERTIFICATES:
                ns = results[provider].get((cert, mode))
                if ns is None:
                    cells.append("Failed")
                    continue
                rank = 1 + sum(
                    other < ns
                    for p in PROVIDERS
                    if (other := results[p].get((cert, mode))) is not None
                )
                cells.append(f"{ns / 1e6:.4f} / {1e9 / ns:.1f} ({rank})")
            lines.append(f"| {provider} | " + " | ".join(cells) + " |")
        lines.append("")

    lines += [
        "## Geometric Mean Performance",
        "",
        "For each mode, each throughput is divided by the best throughput for "
        "that certificate; scores are geometric means across certificates "
        "measured by all providers. Higher is better.",
        "",
        "| Mode | Rank | Provider | Score |",
        "| --- | ---: | --- | ---: |",
    ]
    common_by_mode = {}
    for mode in MODES:
        common = [c for c in CERTIFICATES if all((c, mode) in results[p] for p in PROVIDERS)]
        common_by_mode[mode] = common
        if not common:
            continue
        scores = {}
        for provider in PROVIDERS:
            ratios = [
                min(results[p][(cert, mode)] for p in PROVIDERS)
                / results[provider][(cert, mode)]
                for cert in common
            ]
            scores[provider] = math.exp(sum(math.log(x) for x in ratios) / len(ratios)) * 100
        for provider in sorted(PROVIDERS, key=lambda p: -scores[p]):
            rank = 1 + sum(score > scores[provider] for score in scores.values())
            lines.append(f"| {mode.title()} | {rank} | {provider} | {scores[provider]:.1f}% |")
    for mode in MODES:
        common = common_by_mode[mode]
        lines.append(
            f"\n{mode.title()} common certificates: "
            f"{', '.join(common) if common else 'none; score unavailable'}."
        )

    lines += ["", "## Setup Failures", ""]
    failures = [
        (provider, cert, mode, reason)
        for provider in PROVIDERS
        for (cert, mode), reason in skips[provider].items()
    ]
    if failures:
        lines += ["| Provider | Certificate | Mode | Reason |", "| --- | --- | --- | --- |"]
        for provider, cert, mode, reason in failures:
            lines.append(f"| {provider} | {cert} | {mode} | {escape(reason)} |")
    else:
        lines.append("None.")
    return "\n".join(lines) + "\n"


def check_certificates(cert_dir):
    paths = [cert_dir / "ca-rsa" / "ca.crt"]
    paths += [
        cert_dir / "issued" / "ca-rsa" / f"{cert}.{extension}"
        for cert in CERTIFICATES
        for extension in ("crt", "key")
    ]
    for path in paths:
        if not path.is_file() or path.stat().st_size == 0:
            raise ValueError(f"Missing or empty certificate file: {path}. Run make tls-handshake-certs first.")


def run_provider(provider, criterion_dir, options):
    env = os.environ.copy()
    env["CRITERION_HOME"] = str(criterion_dir)
    command = [
        "cargo", "bench", "--locked", "--manifest-path", str(MANIFEST),
        "--bench", "full_handshake", "--no-default-features", "--features", provider,
    ]
    if options:
        command += ["--", *options]
    print(f"\nRunning {provider}...", flush=True)
    skip_lines = []
    with subprocess.Popen(command, env=env, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT, text=True) as process:
        for line in process.stdout:
            print(line, end="", flush=True)
            if line.startswith("TLS_BENCH_SKIP\t"):
                skip_lines.append(line)
        if process.wait():
            raise subprocess.CalledProcessError(process.returncode, command)
    skips = {}
    for line in skip_lines:
        _, cert, mode, reason = line.rstrip("\n").split("\t", 3)
        if (cert not in CERTIFICATES or mode not in MODES
                or (cert, mode) in skips or not reason.strip()):
            raise ValueError(f"Invalid skip record: {line.strip()}")
        skips[(cert, mode)] = reason
    return collect_results(criterion_dir, provider, skips), skips


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="New report path (must not already exist)")
    parser.add_argument("--sample-size", type=int, help="Criterion sample count (minimum 10)")
    parser.add_argument("--warm-up-time", type=float, help="Criterion warmup seconds")
    parser.add_argument("--measurement-time", type=float, help="Criterion measurement seconds")
    args = parser.parse_args()
    if args.sample_size is not None and args.sample_size < 10:
        parser.error("--sample-size must be at least 10")
    for name in ("warm_up_time", "measurement_time"):
        value = getattr(args, name)
        if value is not None and (not math.isfinite(value) or value <= 0):
            parser.error(f"--{name.replace('_', '-')} must be positive")
    now = datetime.datetime.now(datetime.timezone.utc)
    output = args.output or TLS_DIR / "results" / f"handshake-benchmark-{now:%Y%m%dT%H%M%S%fZ}.md"
    if output.exists():
        raise FileExistsError(f"Report already exists: {output}")
    if not output.parent.is_dir():
        raise FileNotFoundError(f"Report directory does not exist: {output.parent}")
    options = []
    for name in ("sample_size", "warm_up_time", "measurement_time"):
        value = getattr(args, name)
        if value is not None:
            options += [f"--{name.replace('_', '-')}", str(value)]

    check_certificates(TLS_DIR / "certs")
    results, skips = {}, {}
    with tempfile.TemporaryDirectory(prefix="llrt-tls-criterion-") as temp_dir:
        for provider in PROVIDERS:
            results[provider], skips[provider] = run_provider(
                provider, Path(temp_dir) / provider, options
            )
        report = render_report(results, skips, now, options)
    with output.open("x", encoding="utf-8") as file:
        file.write(report)
    print(f"\nReport: {output}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        print(f"Error: {error}", file=sys.stderr)
        sys.exit(1)
