## Introduction

This folder contains unit tests for validating specific modules and functions in
isolation.

## web-platform-tests

A subset of the [web-platform-tests](https://github.com/web-platform-tests/wpt)
are executed against modules that should provide compatibility with existing
standards such as the [WHATWG URL standard](https://url.spec.whatwg.org/) for
`URL` and `URLSearchParams`.

### Test Harness

The web-platform-tests repo exists as a git subtree which can be updated by
running:

```sh
git subtree pull --prefix tests/unit/web-platform-tests https://github.com/web-platform-tests/wpt.git master --squash
```

and resolving any conflicts. Because the web-platform-tests repo is so large,
only the files needed for LLRT should be added or updated; any new files from
the remote should be removed when updating the subtree.

The repo provides a generic JavaScript test harness that uses the global scope
and is designed to run tests in a single run; it has been modified to allow
subsets of tests to be executed separately for better testing and feedback.
