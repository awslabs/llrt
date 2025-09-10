const assert = require("assert");

const utils = require("elem-hono/utils/url");

assert.ok(utils.url() === "foo");
