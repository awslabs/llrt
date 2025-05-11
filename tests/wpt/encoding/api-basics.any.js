// META: global=window,dedicatedworker,shadowrealm
// META: title=Encoding API: Basics

test(function () {
  assert_equals(
    new TextEncoder().encoding,
    "utf-8",
    "default encoding is utf-8"
  );
  assert_equals(
    new TextDecoder().encoding,
    "utf-8",
    "default encoding is utf-8"
  );
}, "Default encodings");

test(function () {
  assert_array_equals(
    new TextEncoder().encode(),
    [],
    "input default should be empty string"
  );
  assert_array_equals(
    new TextEncoder().encode(undefined),
    [],
    "input default should be empty string"
  );
}, "Default inputs");

function testDecodeSample(encoding, string, bytes) {
  test(function () {
    assert_equals(
      new TextDecoder(encoding).decode(new Uint8Array(bytes)),
      string
    );
    assert_equals(
      new TextDecoder(encoding).decode(new Uint8Array(bytes).buffer),
      string
    );
  }, "Decode sample: " + encoding);
}

// z (ASCII U+007A), cent (Latin-1 U+00A2), CJK water (BMP U+6C34),
// G-Clef (non-BMP U+1D11E), PUA (BMP U+F8FF), PUA (non-BMP U+10FFFD)
// byte-swapped BOM (non-character U+FFFE)
var sample = "z\xA2\u6C34\uD834\uDD1E\uF8FF\uDBFF\uDFFD\uFFFE";

test(function () {
  var encoding = "utf-8";
  var string = sample;
  var bytes = [
    0x7a, 0xc2, 0xa2, 0xe6, 0xb0, 0xb4, 0xf0, 0x9d, 0x84, 0x9e, 0xef, 0xa3,
    0xbf, 0xf4, 0x8f, 0xbf, 0xbd, 0xef, 0xbf, 0xbe,
  ];
  var encoded = new TextEncoder().encode(string);
  assert_array_equals([].slice.call(encoded), bytes);
  assert_equals(
    new TextDecoder(encoding).decode(new Uint8Array(bytes)),
    string
  );
  assert_equals(
    new TextDecoder(encoding).decode(new Uint8Array(bytes).buffer),
    string
  );
}, "Encode/decode round trip: utf-8");

testDecodeSample(
  "utf-16le",
  sample,
  [
    0x7a, 0x00, 0xa2, 0x00, 0x34, 0x6c, 0x34, 0xd8, 0x1e, 0xdd, 0xff, 0xf8,
    0xff, 0xdb, 0xfd, 0xdf, 0xfe, 0xff,
  ]
);

testDecodeSample(
  "utf-16be",
  sample,
  [
    0x00, 0x7a, 0x00, 0xa2, 0x6c, 0x34, 0xd8, 0x34, 0xdd, 0x1e, 0xf8, 0xff,
    0xdb, 0xff, 0xdf, 0xfd, 0xff, 0xfe,
  ]
);

testDecodeSample(
  "utf-16",
  sample,
  [
    0x7a, 0x00, 0xa2, 0x00, 0x34, 0x6c, 0x34, 0xd8, 0x1e, 0xdd, 0xff, 0xf8,
    0xff, 0xdb, 0xfd, 0xdf, 0xfe, 0xff,
  ]
);
