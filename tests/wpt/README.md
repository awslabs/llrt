## web-platform-tests

A subset of the [web-platform-tests](https://github.com/web-platform-tests/wpt)
are executed against modules that should provide compatibility with existing
standards such as the [WHATWG URL standard](https://url.spec.whatwg.org/) for
`URL` and `URLSearchParams`.

### Test Harness

The web-platform-tests repo exists as a git subtree which can be updated by
running:

```sh
git subtree pull --prefix tests/wpt https://github.com/web-platform-tests/wpt.git master --squash
```

and resolving any conflicts. Because the web-platform-tests repo is so large,
only the files needed for LLRT should be added or updated; any new files from
the remote should be removed when updating the subtree.

The repo provides a generic JavaScript test harness that uses the global scope
and is designed to run tests in a single run; it has been modified to use an
isolated test context for each test instead of the global scope, allowing tests
to be grouped and executed in parallel. The test harness and test scripts have
been surrounded with closures that provide any "globals" needed. For example, a
web-platform-test like this:

```js
test(() => {
  const a = new URL("https://example.com/");
  assert_equals(JSON.stringify(a), '"https://example.com/"');
});
```

is updated like this:

```js
export default function ({ assert_equals, test }) {
  test(() => {
    const a = new URL("https://example.com/");
    assert_equals(JSON.stringify(a), '"https://example.com/"');
  });
}
```

and the test context is injected when running the tests:

```js
require("./some/test.any.js").default(context);
```

The changes to each file are as minimal as possible to avoid future conflicts
when updating from the web-platform-tests repo.

### Adding Tests

To add new tests, you can either check them out from the repo, or just copy
them, but be sure the path in `tests/wpt` reflects the path
in the web-platform-tests repo.

```sh
git remote add web-platform-tests https://github.com/web-platform-tests/wpt.git
git checkout web-platform-tests/master -- <path>
mv <path> tests/wpt/<path>
```

It's recommended to commit the added files before making any changes so we have
a clear history.

For each test script from web-platform-tests, surround the script in a closure
to provide the "globals" the script expects. Refer to existing tests for
examples. Try to make the minimal changes possible and don't use any
auto-formatting!

If you are using vscode (Visual Studio Code), it is very convenient to open
the `tests/wpt` directory directly, as this will enable the settings
in `.vscode/settings.json` and prevent automatic formatting.
