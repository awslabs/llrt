# Install and deploy

## Configure Lambda functions to use LLRT

Download the latest LLRT release from <https://github.com/awslabs/llrt/releases>

### Option 1: Custom runtime (recommended)

Choose `Custom Runtime on Amazon Linux 2023` and package the LLRT `bootstrap` binary together with your JS code.

### Option 2: Use a layer

Choose `Custom Runtime on Amazon Linux 2023`, upload `llrt-lambda-arm64.zip` or `llrt-lambda-x64.zip` as a layer and add to your function.

### Option 3: Package LLRT in a container image

See our [AWS SAM example](https://github.com/awslabs/llrt/tree/main/example/llrt-sam-container-image) or:

```dockerfile
FROM --platform=arm64 busybox
WORKDIR /var/task/
COPY app.mjs ./
ADD https://github.com/awslabs/llrt/releases/latest/download/llrt-container-arm64 /usr/bin/llrt
RUN chmod +x /usr/bin/llrt

ENV LAMBDA_HANDLER "app.handler"

CMD [ "llrt" ]
```

### Option 4: AWS SAM

The following [example project](https://github.com/awslabs/llrt/tree/main/example/llrt-sam) sets up a lambda instrumented with a layer containing the llrt runtime.

### Option 5: AWS CDK

You can use [`cdk-lambda-llrt` construct library](https://github.com/tmokmss/cdk-lambda-llrt) to deploy LLRT Lambda functions with AWS CDK.

```ts
import { LlrtFunction } from "cdk-lambda-llrt";

const handler = new LlrtFunction(this, "Handler", {
  entry: "lambda/index.ts",
});
```

See [Construct Hub](https://constructs.dev/packages/cdk-lambda-llrt/) and [its examples](https://github.com/tmokmss/cdk-lambda-llrt/tree/main/example) for more details.

## Build from source

1. Clone code and cd to directory

    ```
    git clone git@github.com:awslabs/llrt.git
    cd llrt
    ```

2. Install git submodules

    ```
    git submodule update --init --checkout
    ```

3. Install rust

    ```
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y
    source "$HOME/.cargo/env"
    ```

4. Install dependencies

    ```
    # MacOS
    brew install zig make cmake zstd node corepack

    # Ubuntu
    sudo apt -y install make zstd
    sudo snap install zig --classic --beta

    # Windows WSL2 (requires systemd to be enabled*)
    sudo apt -y install cmake g++ gcc make zip zstd
    sudo snap install zig --classic --beta

    # Windows WSL2 (If Node.js is not yet installed)
    sudo curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/master/install.sh | bash
    nvm install --lts
    ```

    _\* See [Microsoft Devblogs](https://devblogs.microsoft.com/commandline/systemd-support-is-now-available-in-wsl/#how-can-you-get-systemd-on-your-machine)_

5. Install Node.js packages

    ```
    corepack enable
    yarn
    ```

6. Install generate libs and setup rust targets & toolchains

    ```
    make stdlib && make libs
    ```

    !!! note
        If these commands exit with an error that says `can't cd to zstd/lib`,
        you've not cloned this repository recursively. Run `git submodule update --init` to download the submodules and run the commands above again.

7. Build binaries for Lambda (Per bundle type and architecture desired)

    ```
    # for arm64, use
    make llrt-lambda-arm64.zip
    make llrt-lambda-arm64-no-sdk.zip
    make llrt-lambda-arm64-full-sdk.zip
    # or for x86-64, use
    make llrt-lambda-x64.zip
    make llrt-lambda-x64-no-sdk.zip
    make llrt-lambda-x64-full-sdk.zip
    ```

8. Build binaries for Container (Per bundle type and architecture desired)

    ```
    # for arm64, use
    make llrt-container-arm64
    make llrt-container-arm64-no-sdk
    make llrt-container-arm64-full-sdk
    # or for x86-64, use
    make llrt-container-x64
    make llrt-container-x64-no-sdk
    make llrt-container-x64-full-sdk
    ```

9. Optionally build for your local machine (Mac or Linux)

    ```
    make release
    make release-no-sdk
    make release-full-sdk
    ```

You should now have a `llrt-lambda-arm64*.zip` or `llrt-lambda-x64*.zip`. You can manually upload this as a Lambda layer or use it via your Infrastructure-as-code pipeline.

## Crypto and TLS backend options

LLRT supports multiple cryptographic backends for both the crypto module and TLS connections. These can be configured via Cargo features.

### Crypto provider features

| Feature                 | Description                                                       |
| ----------------------- | ----------------------------------------------------------------- |
| `crypto-rust` (default) | Pure Rust crypto using RustCrypto crates                          |
| `crypto-ring`           | Ring-only crypto (limited algorithm support)                      |
| `crypto-ring-rust`      | Ring for hashing/HMAC, RustCrypto for everything else             |
| `crypto-graviola`       | Graviola-only crypto (limited algorithm support)                  |
| `crypto-graviola-rust`  | Graviola for hashing/HMAC/AES-GCM, RustCrypto for everything else |
| `crypto-openssl`        | OpenSSL-based crypto                                              |

### TLS backend features

| Feature              | Description                                   |
| -------------------- | --------------------------------------------- |
| `tls-ring` (default) | rustls with ring crypto                       |
| `tls-aws-lc`         | rustls with AWS-LC crypto (optimized for AWS) |
| `tls-graviola`       | rustls with graviola crypto                   |
| `tls-openssl`        | OpenSSL for TLS                               |

### Building with different backends

```bash
# Default (crypto-rust + tls-ring)
cargo build --release

# Using AWS-LC for TLS
cargo build --release --no-default-features --features "macro,tls-aws-lc"

# Using OpenSSL for both crypto and TLS
cargo build --release --no-default-features --features "macro,crypto-openssl,tls-openssl"

# Using Graviola for both crypto and TLS
cargo build --release --no-default-features --features "macro,crypto-graviola-rust,tls-graviola"
```

## Running Lambda emulator

Please note that in order to run the example you will need:

- Valid AWS credentials via a `~/.aws/credentials` or via environment variables.

```bash
export AWS_ACCESS_KEY_ID=XXX
export AWS_SECRET_ACCESS_KEY=YYY
export AWS_REGION=us-east-1
```

- A DynamoDB table (with `id` as the partition key) on `us-east-1`
- The `dynamodb:PutItem` IAM permission on this table. You can use this policy (don't forget to modify <YOUR_ACCOUNT_ID>):

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "putItem",
      "Effect": "Allow",
      "Action": "dynamodb:PutItem",
      "Resource": "arn:aws:dynamodb:us-east-1:<YOUR_ACCOUNT_ID>:table/quickjs-table"
    }
  ]
}
```

Start the `lambda-server.js` in a separate terminal:

```bash
node lambda-server.js
```

Then run llrt:

```bash
make run
```

## Environment variables

### `LLRT_ASYNC_HOOKS=value`

When using asynchronous hooks, the hooking function inside QuickJS is activated. This is disabled by default as there is concern that it may have a significant impact on performance.

By setting this environment variable to `1`, the asynchronous hook function can be enabled, allowing you to track asynchronous processing using the `async_hooks` module.

### `LLRT_EXTRA_CA_CERTS=file`

Load extra certificate authorities from a PEM encoded file

### `LLRT_GC_THRESHOLD_MB=value`

Set a memory threshold in MB for garbage collection. Default threshold is 20MB

### `LLRT_HTTP_VERSION=value`

Extends the HTTP request version. By default, only HTTP/1.1 is enabled. Specifying '2' will enable HTTP/1.1 and HTTP/2.

### `LLRT_LOG=[target][=][level][,...]`

Filter the log output by target module, level, or both (using `=`). Log levels are case-insensitive and will also enable any higher priority logs.

Log levels in descending priority order:

- `Error`
- `Warn | Warning`
- `Info`
- `Debug`
- `Trace`

Example filters:

- `warn` will enable all warning and error logs
- `llrt_core::vm=trace` will enable all logs in the `llrt_core::vm` module
- `warn,llrt_core::vm=trace` will enable all logs in the `llrt_core::vm` module and all warning and error logs in other modules

### `LLRT_NET_ALLOW="host[ ...]"`

Space-delimited list of hosts or socket paths which should be allowed for network connections. Network connections will be denied for any host or socket path missing from this list. Set an empty list to deny all connections

### `LLRT_NET_DENY="host[ ...]"`

Space-delimited list of hosts or socket paths which should be denied for network connections

### `LLRT_NET_POOL_IDLE_TIMEOUT=value`

Set a timeout in seconds for idle sockets being kept-alive. Default timeout is 15 seconds

### `LLRT_PLATFORM=value`

Used to explicitly specify a preferred platform for the Node.js package resolver. The default is `browser`. If `node` is specified, "node" takes precedence in the search path. If a value other than `browser` or `node` is specified, it will behave as if "browser" was specified.

### `LLRT_REGISTER_HOOKS=file`

If you want to enable a hooking mechanism that is mostly compatible with Node.js's `module.registerHooks()`, specify the js file name in this environment variable.

We provide a concrete example in `example/register-hooks`.

!!! note
    This environment variable is only effective when running on AWS Lambda.
    When using the LLRT CLI, hook files must be specified using the --import option instead of this environment variable.

### `LLRT_SDK_CONNECTION_WARMUP=1`

Initializes TLS connections in parallel during function init which significantly reduces cold starts due. Enabled by default, can be disabled with value `0` or `false`

### `LLRT_TLS_VERSION=value`

Set the TLS version to be used for network connections. By default only TLS 1.2 is enabled. TLS 1.3 can also be enabled by setting this variable to `1.3`
