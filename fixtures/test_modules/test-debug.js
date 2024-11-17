const assert = require("assert");

const debug = require("elem-debug");

assert.ok(debug.cat() == "cat");
assert.ok(debug.length() == 0);
