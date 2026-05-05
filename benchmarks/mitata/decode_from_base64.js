import { run, bench, group, summary } from "mitata";

import { decodeFromBase64 } from "llrt:codec";

const input1 = "dXNlcjpwYXNz";
console.log(Buffer.from(input1, "base64"));
console.log(decodeFromBase64(input1));
console.log(Uint8Array.fromBase64(input1));

[
  ["small", "dXNlcjpwYXNz"],
  [
    "middle",
    "aGVsbG8gd29ybGQgdGhpcyBpcyBhIGxvbmdlciBiYXNlNjQgc3RyaW5nIGZvciB0ZXN0aW5n",
  ],
  [
    "large",
    "VGhpcyBpcyBhIGxvbmcgYmFzZTY0IHN0cmluZyBleGFtcGxlIHRoYXQgcmVwcmVzZW50cyBhIGxhcmdlciBwYXlsb2FkLiBUaGlzIGlzIHVzZWQgdG8gc2ltdWxhdGUgYSBiaWdnZXIgZGF0YSBibG9iIGluIGEgcmVhbGlzdGljIHNjZW5hcmlvLg==",
  ],
].forEach(([type, input]) => {
  group(type, () => {
    summary(() => {
      bench("Buffer.from()", () => {
        Buffer.from(input, "base64");
      });
      bench("llrt:codec.decodeFromBase64()", () => {
        decodeFromBase64(input);
      });
      bench("Uint8Array.fromBase64()", () => {
        Uint8Array.fromBase64(input);
      });
    });
  });
});

await run();
