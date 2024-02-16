FROM node:18 as build
WORKDIR /llrt/example/llrt-sam

# install aws sam
RUN <<EOF
apt-get update
apt install wget
wget https://github.com/aws/aws-sam-cli/releases/latest/download/aws-sam-cli-linux-x86_64.zip
unzip aws-sam-cli-linux-x86_64.zip -d sam-installation
./sam-installation/install
EOF

COPY . .

RUN <<EOF
wget -P ../../ https://github.com/awslabs/llrt/releases/download/v0.1.6-beta/llrt-lambda-arm64.zip
npm install -g esbuild
sam build
#sam deploy --guided
EOF
