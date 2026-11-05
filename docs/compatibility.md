# Compatibility

!!! note
    LLRT only supports a fraction of the Node.js APIs. It is **NOT** a drop in replacement for Node.js, nor will it ever be. Below is a high level overview of partially supported APIs and modules. For more details consult the [API](https://github.com/awslabs/llrt/blob/main/API.md) documentation.

## Node.js API

| [Node.js API](https://nodejs.org/api/index.html) | Node.js | LLRT        |
| ------------------------------------------------ | ------- | ----------- |
| node:assert                                      | Yes     | Partial     |
| node:async_hooks                                 | Yes     | Partial     |
| node:buffer                                      | Yes     | Partial     |
| node:child_process                               | Yes     | Partial     |
| node:cluster                                     | Yes     | No          |
| node:console                                     | Yes     | Partial     |
| node:crypto                                      | Yes     | Partial     |
| node:dgram                                       | Yes     | No          |
| node:diagnostics_channel                         | Yes     | No          |
| node:dns                                         | Yes     | Partial     |
| node:events                                      | Yes     | Partial     |
| node:fs                                          | Yes     | Partial     |
| node:fs/promises                                 | Yes     | Partial     |
| node:http                                        | Yes     | Planned     |
| node:http2                                       | Yes     | No          |
| node:https                                       | Yes     | Planned     |
| node:inspector                                   | Yes     | No          |
| node:inspector/promises                          | Yes     | No          |
| node:module                                      | Yes     | Partial     |
| node:net                                         | Yes     | Partial     |
| node:os                                          | Yes     | Partial     |
| node:path                                        | Yes     | Partial     |
| node:perf_hooks                                  | Yes     | Partial     |
| node:process                                     | Yes     | Partial     |
| node:querystring                                 | Yes     | No          |
| node:readline                                    | Yes     | No          |
| node:readline/promises                           | Yes     | No          |
| node:repl                                        | Yes     | No          |
| node:sqlite                                      | Yes     | No          |
| node:stream                                      | Yes\*   | Partial\*   |
| node:stream/promises                             | Yes\*   | Partial\*   |
| node:stream/web                                  | Yes     | Partial     |
| node:string_decoder                              | Yes     | Yes         |
| node:test                                        | Yes     | No          |
| node:timers                                      | Yes     | Partial     |
| node:tls                                         | Yes     | Planned     |
| node:tty                                         | Yes     | Partial     |
| node:url                                         | Yes     | Partial     |
| node:util                                        | Yes     | Partial     |
| node:v8                                          | Yes     | No\*\*      |
| node:vm                                          | Yes     | No          |
| node:wasi                                        | Yes     | No          |
| node:worker_threads                              | Yes     | No          |
| node:zlib                                        | Yes     | Partial     |

## LLRT API

| [LLRT API](https://github.com/awslabs/llrt/blob/main/API.md) | Node.js | LLRT |
| ------------------------------------------------------------ | ------- | ---- |
| llrt:codec                                                   | No      | Yes  |
| llrt:qjs                                                     | No      | Yes  |
| llrt:util                                                    | No      | Yes  |
| llrt:xml                                                     | No      | Yes  |

## Web Platform API

| [Web Platform API](https://min-common-api.proposal.wintertc.org/) | LLRT    |
| ----------------------------------------------------------------- | ------- |
| COMPRESSION                                                       | Planned |
| CONSOLE                                                           | Partial |
| DOM                                                               | Partial |
| ECMASCRIPT                                                        | Partial |
| ENCODING                                                          | Partial |
| FETCH                                                             | Partial |
| FILEAPI                                                           | Partial |
| HR-TIME                                                           | Yes     |
| HTML                                                              | Partial |
| STREAMS                                                           | Yes     |
| URL                                                               | Yes     |
| URLPATTERN                                                        | Planned |
| WASM-JS-API-2                                                     | No      |
| WASM-WEB-API-2                                                    | No      |
| WEBCRYPTO                                                         | Partial |
| WEBIDL                                                            | Partial |
| XHR                                                               | Partial |

## Other features

| Other features | LLRT    |
| -------------- | ------- |
| async/await    | Yes     |
| esm            | Yes     |
| cjs            | Yes     |
| Intl           | Partial |
| Temporal       | Partial |

- Partial: partially supported in LLRT
- Planned: planned partial support
- \* Not native
- \*\* The `module.registerHooks()` API allows you to emulate some functionality. See also `example/register-hooks`.

## Testing

The best way to ensure your code is compatible with LLRT is to write tests and execute them using the built-in test runner. The test runner currently supports Jest/Chai assertions. There are three main types of tests you can create:

**Unit tests**

- Useful for validating specific modules and functions in isolation
- Allow focused testing of individual components

**End-to-End (E2E) tests**

- Validate overall compatibility with AWS SDK and WinterTC compliance
- Test the integration between all components
- Confirm expected behavior from end-user perspective
- For more information about the E2E tests and how to run them, see [here](https://github.com/awslabs/llrt/blob/main/tests/e2e/README.md).

**Web Platform Tests (WPT)**

- Useful for validating LLRT's behavior against standardized browser APIs and runtime expectations
- Ensure compatibility with web standards and cross-runtime environments
- Help verify alignment with WinterTC and broader JavaScript ecosystem
- For setup instructions and how to run WPT in LLRT, see [here](https://github.com/awslabs/llrt/blob/main/tests/wpt/README.md).

### Test runner

Test runner uses a lightweight Jest-like API and supports Jest/Chai assertions. For examples on how to implement tests for LLRT see the [`tests`](https://github.com/awslabs/llrt/tree/main/tests) folder of this repository.

To run tests, execute the `llrt test` command. LLRT scans the current directory and sub-directories for files that ends with `*.test.js` or `*.test.mjs`. You can also provide a specific test directory to scan by using the `llrt test -d <directory>` option.

The test runner also has support for filters. Using filters is as simple as adding additional command line arguments, i.e: `llrt test crypto` will only run tests that match the filename containing `crypto`.
