const assert = require("assert");

import * as jmespath1 from "elem-aws-lambda-powertools/jmespath";
assert.ok(typeof jmespath1.isNull === "function");

const jmespath2 = require("elem-aws-lambda-powertools/jmespath");
assert.ok(typeof jmespath2.isNull === "function");
