# LLRT TLS Handshake Benchmark

This directory contains an in-memory full and resumed TLS 1.2 handshake
benchmark for the rustls-based LLRT TLS providers.

```sh
make tls-handshake-certs
python3 benchmarks/tls/scripts/run-handshake-bench.py
```

Generate the test certificates before running the benchmark. The script runs all
four providers sequentially and saves a timestamped Markdown report under
`benchmarks/tls/results/`; it does not generate certificates.
Use `--output PATH` to select a new report file; existing files are not
overwritten. Python 3, OpenSSL, Cargo and a Rust toolchain are required.

See [handshake-bench/README.md](handshake-bench/README.md) for individual
provider commands and benchmark details.
