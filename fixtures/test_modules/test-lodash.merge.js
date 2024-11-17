const assert = require("assert");

import merge1 from "elem-lodash.merge";
assert.ok(typeof merge1 === "function");

const merge2 = require("elem-lodash.merge");
assert.ok(typeof merge2 === "function");
