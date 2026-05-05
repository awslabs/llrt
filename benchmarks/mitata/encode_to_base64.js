import { run, bench, group, summary } from "mitata";

import { encodeToBase64 } from "llrt:codec";

const ENCODER = new TextEncoder();

function hyblidToBase64(input) {
  return input instanceof Uint8Array ? input.toBase64() : encodeToBase64(input);
}

const input1 = Buffer.from("hello world");
console.log(Buffer.from(input1).toString("base64"));
console.log(encodeToBase64(input1));
console.log(input1.toBase64());

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
      bench("Buffer.from().toString()", () => {
        Buffer.from(input).toString("base64");
      });
      bench("llrt:codec.encodeToBase64()", () => {
        encodeToBase64(input);
      });
      bench("Uint8Array.toBase64()", () => {
        input.toBase64();
      });
    });
  });
});

const input2 = "hello world";
console.log(Buffer.from(input2).toString("base64"));
console.log(encodeToBase64(input2));

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
        Buffer.from(input).toString("base64");
      });
      bench("llrt:codec.encodeToBase64()", () => {
        encodeToBase64(input);
      });
    });
  });
});

await run();
