#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
TLS_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
CERT_DIR="$TLS_DIR/certs"
ISSUER_DIR="$CERT_DIR/ca-rsa"
ISSUED_DIR="$CERT_DIR/issued/ca-rsa"

command -v openssl >/dev/null 2>&1 || {
  echo "OpenSSL is required." >&2
  exit 1
}

mkdir -p "$ISSUER_DIR" "$ISSUED_DIR"
umask 077

if [[ ! -s "$ISSUER_DIR/ca.crt" || ! -s "$ISSUER_DIR/ca.key" ]]; then
  openssl req -new -x509 -newkey rsa:2048 -nodes \
    -keyout "$ISSUER_DIR/ca.key" -out "$ISSUER_DIR/ca.crt" \
    -days 3650 -sha256 -subj "/CN=LLRT TLS Handshake Benchmark CA" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,digitalSignature,keyCertSign,cRLSign"
fi

for leaf_id in rsa-2048 rsa-3072 rsa-4096 ecdsa-p256 ecdsa-p384 ecdsa-p521 ed25519; do
  key="$ISSUED_DIR/$leaf_id.key"
  cert="$ISSUED_DIR/$leaf_id.crt"
  csr="$ISSUED_DIR/$leaf_id.csr"
  if [[ -s "$key" && -s "$cert" ]]; then
    continue
  fi

  case "$leaf_id" in
    rsa-2048) openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$key" ;;
    rsa-3072) openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:3072 -out "$key" ;;
    rsa-4096) openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:4096 -out "$key" ;;
    ecdsa-p256) openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:prime256v1 -out "$key" ;;
    ecdsa-p384) openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:secp384r1 -out "$key" ;;
    ecdsa-p521) openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:secp521r1 -out "$key" ;;
    ed25519) openssl genpkey -algorithm ED25519 -out "$key" ;;
  esac

  openssl req -new -key "$key" -out "$csr" -subj "/CN=localhost"
  openssl x509 -req -in "$csr" \
    -CA "$ISSUER_DIR/ca.crt" -CAkey "$ISSUER_DIR/ca.key" -CAcreateserial \
    -out "$cert" -days 365 -sha256 \
    -extfile "$SCRIPT_DIR/server-cert-ext.cnf" -extensions server
  rm -f "$csr"
done
