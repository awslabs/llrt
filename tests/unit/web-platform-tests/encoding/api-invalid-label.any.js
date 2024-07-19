// META: title=Encoding API: invalid label
// META: timeout=long
// META: variant=?1-1000
// META: variant=?1001-2000
// META: variant=?2001-3000
// META: variant=?3001-last
// META: script=resources/encodings.js
// META: script=/common/subset-tests.js

export default function({
  assert_throws_js,
  encodings_table,
  format_value,
  setup,
  subsetTest,
  test,
}) {

var tests = ["invalid-invalidLabel"];
setup(function() {
  encodings_table.forEach(function(section) {
    section.encodings.forEach(function(encoding) {
      encoding.labels.forEach(function(label) {
        ["\u0000", "\u000b", "\u00a0", "\u2028", "\u2029"].forEach(function(ws) {
          tests.push(ws + label);
          tests.push(label + ws);
          tests.push(ws + label + ws);
        });
      });
    });
  });
});

tests.forEach(function(input) {
  subsetTest(test, function() {
    assert_throws_js(RangeError, function() { new TextDecoder(input); });
  }, 'Invalid label ' + format_value(input) + ' should be rejected by TextDecoder.');
});

};
