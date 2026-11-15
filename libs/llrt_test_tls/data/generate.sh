#! /bin/bash

# Usage: ./generate.sh          regenerates the server and client certificates
#        ./generate.sh client   regenerates only the client certificates

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

  # Clean up temporary files
  rm client.csr client-ca.key client-ca.srl client-extensions.cnf
}

if [ "$1" = "client" ]; then
  generate_client
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
