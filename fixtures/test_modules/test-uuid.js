const assert = require("assert");

var fn = _interopRequireDefault(require("elem-uuid"));
function _interopRequireDefault(e) {
  return e && e.__esModule ? e : { default: e };
}
var greeting = (0, fn.default)("hello");

assert.ok(greeting("world") == "hello, world");
