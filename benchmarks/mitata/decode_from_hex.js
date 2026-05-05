import { run, bench, group, summary } from "mitata";

import { decodeFromHex } from "llrt:codec";

const input1 = "cafed00d";
console.log(Buffer.from(input1, "hex"));
console.log(decodeFromHex(input1));
console.log(Uint8Array.fromHex(input1));

[
  ["small", "cafed00d"],
  [
    "middle",
    "48656c6c6f20776f726c6420746869732069732061206c6f6e6765722068657820737472",
  ],
  [
    "large",
    "48656c6c6f20776f726c6420746869732069732061206c6f6e676572206865782073747248656c6c6f20776f726c6420746869732069732061206c6f6e6765722068657820737472",
  ],
].forEach(([type, input]) => {
  group(type, () => {
    summary(() => {
      // bench("Buffer.from()", () => {
      //   Buffer.from(input, "hex");
      // });
      bench("llrt:codec.decodeFromHex()", () => {
        decodeFromHex(input);
      });
      bench("Uint8Array.fromHex()", () => {
        Uint8Array.fromHex(input);
      });
    });
  });
});

await run();
