import hex from "hex";

it("should encode/decode text", () => {
  let hello = "hello";
  const encoded = new TextEncoder().encode(hello);
  const decoded = new TextDecoder().decode(encoded);

  assert.equal(decoded, hello);
});

it("should encode/decode hex", () => {
  const byteArray = new TextEncoder().encode("hello");
  const encoded = hex.encode(byteArray);

  assert.equal(encoded, "68656c6c6f");
});
