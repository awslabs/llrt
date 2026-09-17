import { runSuite, runTestDynamic } from "./url.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "historical.any.js", // TypeError: cannot read property 'isWindow' of undefined
  "idlharness.any.js", // ReferenceError: idl_test is not defined
  [
    "IdnaTestV2.any.js",
    [/ToASCII\("[^\"]*[xX][nN]--[^\"]*"\)/], // Blocked by servo/rust-url#1135: invalid ASCII xn-- labels are rejected.
  ],
  [
    "url-constructor.any.js",
    [/Parsing: <[^>]*[xX][nN]--[^>]*> without base/], // Blocked by servo/rust-url#1135: invalid ASCII xn-- labels are rejected.
  ],
  [
    "url-origin.any.js",
    [/Origin parsing: <[^>]*[xX][nN]--[^>]*> without base/], // Blocked by servo/rust-url#1135: invalid ASCII xn-- labels are rejected.
  ],
  [
    "url-setters.any.js",
    [/Setting <[^>]*>\.(?:host|hostname) = 'xn--/], // Blocked by servo/rust-url#1135: invalid ASCII xn-- labels are rejected.
  ],
]);
