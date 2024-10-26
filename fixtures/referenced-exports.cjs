exports = module.exports;

exports.str = "str";
exports.cat = function cat() {
  return exports.str;
};

exports.array = [1];
exports.length = function length() {
  return exports.array.length;
};
