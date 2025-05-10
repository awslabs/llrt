export default function({
  assert_equals,
  fetch,
  promise_test,
  test,
}) {

promise_test(() => fetch("resources/urltestdata.json").then(res => res.json()).then(runURLTests), "Loading data…");

function runURLTests(urlTests) {
  for (const expected of urlTests) {
    // Skip comments and tests without "origin" expectation
    if (typeof expected === "string" || !("origin" in expected))
      continue;

    // These tests don't pass with current Url crate
    const skip = [
      "blob:blob:https://example.org/",
      "blob:ftp://host/path",
      "blob:ws://example.org/",
      "blob:wss://example.org/",
    ];
    if (skip.includes(expected.input)) continue;

    const base = expected.base !== null ? expected.base : undefined;

    test(() => {
      const url = new URL(expected.input, base);
      assert_equals(url.origin, expected.origin, "origin");
    }, `Origin parsing: <${expected.input}> ${base ? "against <" + base + ">" : "without base"}`);
  }
}

};
