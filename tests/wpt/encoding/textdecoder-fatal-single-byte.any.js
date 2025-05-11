// META: timeout=long
// META: title=Encoding API: Fatal flag for single byte encodings
// META: timeout=long
// META: variant=?1-1000
// META: variant=?1001-2000
// META: variant=?2001-3000
// META: variant=?3001-4000
// META: variant=?4001-5000
// META: variant=?5001-6000
// META: variant=?6001-7000
// META: variant=?7001-last
// META: script=/common/subset-tests.js

var singleByteEncodings = [
  { encoding: "IBM866", bad: [] },
  { encoding: "ISO-8859-2", bad: [] },
  { encoding: "ISO-8859-3", bad: [0xa5, 0xae, 0xbe, 0xc3, 0xd0, 0xe3, 0xf0] },
  { encoding: "ISO-8859-4", bad: [] },
  { encoding: "ISO-8859-5", bad: [] },
  {
    encoding: "ISO-8859-6",
    bad: [
      0xa1, 0xa2, 0xa3, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, 0xae, 0xaf,
      0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xbc,
      0xbd, 0xbe, 0xc0, 0xdb, 0xdc, 0xdd, 0xde, 0xdf, 0xf3, 0xf4, 0xf5, 0xf6,
      0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff,
    ],
  },
  { encoding: "ISO-8859-7", bad: [0xae, 0xd2, 0xff] },
  {
    encoding: "ISO-8859-8",
    bad: [
      0xa1, 0xbf, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9,
      0xca, 0xcb, 0xcc, 0xcd, 0xce, 0xcf, 0xd0, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5,
      0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde, 0xfb, 0xfc, 0xff,
    ],
  },
  {
    encoding: "ISO-8859-8-I",
    bad: [
      0xa1, 0xbf, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9,
      0xca, 0xcb, 0xcc, 0xcd, 0xce, 0xcf, 0xd0, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5,
      0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde, 0xfb, 0xfc, 0xff,
    ],
  },
  { encoding: "ISO-8859-10", bad: [] },
  { encoding: "ISO-8859-13", bad: [] },
  { encoding: "ISO-8859-14", bad: [] },
  { encoding: "ISO-8859-15", bad: [] },
  { encoding: "ISO-8859-16", bad: [] },
  { encoding: "KOI8-R", bad: [] },
  { encoding: "KOI8-U", bad: [] },
  { encoding: "macintosh", bad: [] },
  {
    encoding: "windows-874",
    bad: [0xdb, 0xdc, 0xdd, 0xde, 0xfc, 0xfd, 0xfe, 0xff],
  },
  { encoding: "windows-1250", bad: [] },
  { encoding: "windows-1251", bad: [] },
  { encoding: "windows-1252", bad: [] },
  { encoding: "windows-1253", bad: [0xaa, 0xd2, 0xff] },
  { encoding: "windows-1254", bad: [] },
  {
    encoding: "windows-1255",
    bad: [0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde, 0xdf, 0xfb, 0xfc, 0xff],
  },
  { encoding: "windows-1256", bad: [] },
  { encoding: "windows-1257", bad: [0xa1, 0xa5] },
  { encoding: "windows-1258", bad: [] },
  { encoding: "x-mac-cyrillic", bad: [] },
];

singleByteEncodings.forEach(function (t) {
  for (var i = 0; i < 256; ++i) {
    if (t.bad.indexOf(i) != -1) {
      subsetTest(
        test,
        function () {
          assert_throws_js(TypeError, function () {
            new TextDecoder(t.encoding, { fatal: true }).decode(
              new Uint8Array([i])
            );
          });
        },
        "Throw due to fatal flag: " +
          t.encoding +
          " doesn't have a pointer " +
          i
      );
    } else {
      subsetTest(
        test,
        function () {
          assert_equals(
            typeof new TextDecoder(t.encoding, { fatal: true }).decode(
              new Uint8Array([i])
            ),
            "string"
          );
        },
        "Not throw: " + t.encoding + " has a pointer " + i
      );
    }
  }
});
