## Introduction

This folder contains integration tests that can be run in the LLRT runtime or in a standard Node.js environment using [vitest](https://vitest.dev/). The tests are designed to be environment-agnostic so that behavior can be compared when running the same code in LLRT versus Node.js.

By leveraging vitest's integration with Typescript, these tests can easily be executed outside the LLRT runtime. This allows validation that the code functions the same in LLRT as it does in a typical Node.js runtime.

The goal is to have a suite of integration tests that provide confidence the code will work correctly regardless of whether LLRT or native Node.js is used as the runtime. Running the tests in different environments ensures compatibility and consistent behavior.

## Integration test Prerequisites

Certain resources need to be created to make sure the integration test has backend resources to test against. Follow steps bellow to create them and make them available through env variables:

1. Deploy a CloudFormation stack called `LlrtReleaseIntegTestResourcesStack` using the provided template. This will create the necessary resources for the tests:

   ```shell
   aws cloudformation deploy --stack-name LLRTReleaseIntegTestResourcesStack --template-file ./IntegTestResourcesStack.template.yml --capabilities CAPABILITY_IAM
   ```

2. If you have `jq` [installed](https://jqlang.github.io/jq/), you can use the command below to export env variables for ressources that will be used during the E2E tests.

   ```shell
   $(aws cloudformation describe-stacks --stack-name LLRTReleaseIntegTestResourcesStack --query "Stacks[*].Outputs[*].{OutputKey: OutputKey, OutputValue: OutputValue}" | jq -r '.[0][] | "export \(.OutputKey|gsub("(?<x>(?!^)|\b[a-zA-Z][a-z]*)(?<y>[A-Z][a-z]*|\\d+)";"\(.x)_\(.y)")| ascii_upcase )=\(.OutputValue)"')
   ```

   If `jq` is not available, look at the CloudFormation `Outputs` and manually export the variables:

   - AWS_SMOKE_TEST_BUCKET
   - AWS_SMOKE_TEST_IDENTITY_POOL_ID
   - AWS_SMOKE_TEST_MRAP_ARN

3. Export your AWS credentials and Region:

   ```shell
   export AWS_ACCESS_KEY_ID=XXX
   export AWS_SECRET_ACCESS_KEY=YYY
   export AWS_REGION=ZZZ
   ```

4. Run the integration tests:

   This will build and run all integration tests

   ```shell
   make test-ci
   ```

   Assuming you already have a binary built for LLRT, this will run a specific integration tests (e.g.: `s3.e2e.test.ts`)

   ```shell
   make bundle/%.js && target/debug/llrt  test s3.e2e.test
   ```
