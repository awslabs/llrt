#! /bin/bash

# Usage: ./generate.sh          regenerates the server and client certificates and the keys
#        ./generate.sh client   regenerates only the client certificates
#        ./generate.sh keys     regenerates only the encrypted key fixtures

# Client certificates for mutual TLS tests. They have their own CA, so they
# can be regenerated without changing the server certificates.
generate_client() {
  # Generate the Client CA private key and certificate
  openssl req -x509 -newkey rsa:2048 -keyout client-ca.key -out client-ca.pem -days 3650 -nodes -subj "/CN=Test Client CA"

  # Generate the client private key and CSR
  openssl genrsa -out client.key 2048
  openssl req -new -key client.key -out client.csr -subj "/CN=Test Client"

  # Sign the client certificate with the Client CA, for client authentication only
  echo "extendedKeyUsage = clientAuth" > client-extensions.cnf
  openssl x509 -req -in client.csr -CA client-ca.pem -CAkey client-ca.key -CAcreateserial -out client.pem -days 3650 -sha256 -extfile client-extensions.cnf

  # The client key encrypted with a passphrase: PKCS#8 with OpenSSL's defaults,
  # and the traditional OpenSSL format
  openssl pkcs8 -topk8 -in client.key -passout pass:pässphrase -out client-encrypted.key
  openssl rsa -in client.key -traditional -aes256 -passout pass:pässphrase -out client-encrypted-traditional.key

  # Clean up temporary files
  rm client.csr client-ca.key client-ca.srl client-extensions.cnf
}

# A small EC key, encrypted as PKCS#8 with each PBKDF2 PRF and AES key size,
# with ciphers LLRT doesn't support yet, with an empty passphrase, and with
# legacy algorithms
generate_encrypted_keys() {
  openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out ec.key

  : > ec-encrypted.pem
  for options in \
    "aes-256-cbc hmacWithSHA1" "aes-128-cbc hmacWithMD5" "aes-192-cbc hmac-md5" "aes-256-cbc hmac-sha1" \
    "aes-128-cbc hmacWithSHA224" "aes-192-cbc hmacWithSHA256" "aes-256-cbc hmacWithSHA384" \
    "aes-128-cbc hmacWithSHA512" "aes-192-cbc hmacWithSHA512-224" "aes-256-cbc hmacWithSHA512-256" \
    "aes-128-cbc hmac-sha3-224" "aes-192-cbc hmac-sha3-256" "aes-256-cbc hmac-sha3-384" "aes-128-cbc hmac-sha3-512" \
    "id-aes128-wrap-pad hmacWithSHA256" "des-ede hmacWithSHA256"; do
    set -- $options
    echo "$1 $2" >> ec-encrypted.pem
    openssl pkcs8 -topk8 -in ec.key -v2 "$1" -v2prf "$2" -passout pass:pässphrase >> ec-encrypted.pem
  done

  openssl pkcs8 -topk8 -in ec.key -passout pass: -out ec-encrypted-empty.key

  : > ec-encrypted-legacy.pem
  for algorithm in PBE-MD5-DES PBE-SHA1-RC2-40 PBE-SHA1-RC4-128; do
    echo "$algorithm" >> ec-encrypted-legacy.pem
    openssl pkcs8 -topk8 -in ec.key -v1 "$algorithm" -provider legacy -provider default -passout pass:pässphrase >> ec-encrypted-legacy.pem
  done
}

if [ "$1" = "client" ]; then
  generate_client
  exit 0
fi

if [ "$1" = "keys" ]; then
  generate_encrypted_keys
  exit 0
fi

# Generate the Root CA private key and certificate
openssl req -x509 -newkey rsa:4096 -keyout root.key -out root.pem -days 3650 -nodes -subj "/CN=Test Root CA"

# Generate the server private key
openssl genrsa -out server.key 2048

# Generate a certificate signing request (CSR) for the server
openssl req -new -key server.key -out server.csr -subj "/CN=localhost"

# Create a configuration file for certificate extensions
cat > extensions.cnf << EOF
subjectAltName = IP:127.0.0.1, DNS:localhost
EOF

# Sign the server certificate with the Root CA, including the SAN extension
openssl x509 -req -in server.csr -CA root.pem -CAkey root.key -CAcreateserial -out server.pem -days 3650 -sha256 -extfile extensions.cnf

# Clean up temporary files
rm server.csr root.key root.srl extensions.cnf

generate_client
generate_encrypted_keys
