## Introduction

This folder contains integration tests that can be run in the LLRT runtime or in a standard Node.js environment using [vitest](https://vitest.dev/). The tests are designed to be environment-agnostic so that behavior can be compared when running the same code in LLRT versus Node.js.

By leveraging vitest's integration with Typescript, these tests can easily be executed outside the LLRT runtime. This allows validation that the code functions the same in LLRT as it does in a typical Node.js runtime.

The goal is to have a suite of integration tests that provide confidence the code will work correctly regardless of whether LLRT or native Node.js is used as the runtime. Running the tests in different environments ensures compatibility and consistent behavior.

## Running E2E tests

The simplest way to run the E2E tests is:

```shell
make test-e2e
```

This will:

1. Deploy the CloudFormation stack with required AWS resources (S3 bucket, Cognito Identity Pool, MRAP)
2. Export environment variables from the stack outputs and AWS credentials
3. Build the JS bundles
4. Run the tests

### Prerequisites

- AWS CLI configured with valid credentials
- `jq` is **not** required — the Makefile handles environment setup automatically

### Manual setup

If you prefer to set up manually:

1. Deploy the CloudFormation stack:

   ```shell
   make setup-e2e
   ```

2. Export environment variables:

   ```shell
   eval $(make e2e-env)
   ```

3. Run a specific test (assuming LLRT is already built):

   ```shell
   make bundle/js/%.js && target/debug/llrt test s3.e2e.test
   ```
