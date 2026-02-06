# LLRT Rustls Full Handshake Benchmark

This benchmark measures TLS 1.2 full and resumed rustls handshakes in memory.
It uses LLRT's crypto-provider feature and certificates generated under
`benchmarks/tls/certs/ca-rsa`.

The benchmark creates a new client and server connection for every iteration
and exchanges rustls TLS records in memory. Full handshakes disable resumption;
resumed handshakes reuse a session cache primed before measurement. Both peers'
handshake kinds are verified.

The benchmark matrix matches the TLS client benchmark summary:

- TLS 1.2 only
- Full handshakes and resumed handshakes
- Server authentication only
- RSA 2048/3072/4096
- ECDSA P-256/P-384/P-521
- Ed25519

Setup failures are reported with their reasons and omitted from that provider's
measurements. They do not necessarily imply that the client alone lacks support,
because the selected provider is used on both client and server.

To run all providers sequentially and generate a Markdown report:

```sh
make tls-handshake-certs
python3 benchmarks/tls/scripts/run-handshake-bench.py
```

Generate the certificates before running the script; it does not generate them.
By default, the report is saved under `benchmarks/tls/results/`. Use `--output PATH`
to select a new report file, or `--help` for Criterion timing options. The
script uses a fresh temporary Criterion output directory for each run, so
previous results cannot appear in the report. It reports Criterion's slope
estimate (or mean if no slope is available), not the median.

Generate certificates once, then run one provider at a time:

```sh
make tls-handshake-certs
cargo bench --manifest-path benchmarks/tls/handshake-bench/Cargo.toml --no-default-features --features tls-rust
cargo bench --manifest-path benchmarks/tls/handshake-bench/Cargo.toml --no-default-features --features tls-ring
cargo bench --manifest-path benchmarks/tls/handshake-bench/Cargo.toml --no-default-features --features tls-aws-lc
cargo bench --manifest-path benchmarks/tls/handshake-bench/Cargo.toml --no-default-features --features tls-graviola
```

Or use the Makefile default:

```sh
make tls-handshake-bench
TLS_HANDSHAKE_FEATURE=tls-aws-lc make tls-handshake-bench
```
