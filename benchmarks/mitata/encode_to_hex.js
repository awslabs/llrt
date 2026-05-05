import { run, bench, group, summary } from "mitata";

import { encodeToHex } from "llrt:codec";

function hyblidToHex(input) {
  return input instanceof Uint8Array ? input.toHex() : encodeToHex(input);
}

const input1 = Buffer.from("hello world");
console.log(Buffer.from(input1).toString("hex"));
console.log(encodeToHex(input1));
console.log(input1.toHex());

[
  ["small", Buffer.from("hello world")],
  [
    "middle",
    Buffer.from(
      "hello world this is a longer buffer example for iteration testing"
    ),
  ],
  [
    "large",
    Buffer.from(
      "hello world this is a longer buffer example for iteration testing, hello world this is a longer buffer example for iteration testing"
    ),
  ],
].forEach(([type, input]) => {
  group(type, () => {
    summary(() => {
      // bench("Buffer.from().toString()", () => {
      //   Buffer.from(input).toString("hex");
      // });
      bench("llrt:codec.encodeToHex()", () => {
        encodeToHex(input);
      });
      bench("Uint8Array.toHex()", () => {
        input.toHex();
      });
    });
  });
});

const input2 = "hello world";
console.log(Buffer.from(input2).toString("hex"));
console.log(encodeToHex(input2));

[
  ["small", "hello world"],
  [
    "middle",
    "hello world this is a longer buffer example for iteration testing",
  ],
  [
    "large",
    "hello world this is a longer buffer example for iteration testing, hello world this is a longer buffer example for iteration testing",
  ],
].forEach(([type, input]) => {
  group(type, () => {
    summary(() => {
      bench("Buffer.from().toString()", () => {
        Buffer.from(input).toString("hex");
      });
      bench("llrt:codec.encodeToHex()", () => {
        encodeToHex(input);
      });
    });
  });
});

await run();
