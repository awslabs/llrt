## web-platform-tests

A subset of the [web-platform-tests](https://github.com/web-platform-tests/wpt)
are executed against modules that should provide compatibility with existing
standards such as the [WHATWG URL standard](https://url.spec.whatwg.org/) for
`URL` and `URLSearchParams`.

### Running WPTs

LLRT includes built-in support for running web-platform-tests using a simplified workflow. To get started:

1. **Change sparse-checkout mode** (only once)

```sh
git sparse-checkout init --no-cone
```

2. **Update sparse-checkout information** (only once)

```sh
make init-wpt
```

3. **Load as a submodule** (only once)

```sh
git submodule add --force -b master https://github.com/web-platform-tests/wpt wpt
```

4. **Update the local revision** (when the remote is updated)

```sh
make update-wpt
```

5. **Run the WPTs**

```sh
make test-wpt
```

6. **Organizing the results of the WPT**

```sh
make tidyup-wpt
```
