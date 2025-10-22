#! /bin/bash

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
