describe("Buffer.alloc", () => {
  it("should create a buffer with specified size", () => {
    const size = 10;
    const buffer = Buffer.alloc(size);

    assert.strictEqual(buffer.length, size);
  });

  it("should fill the buffer with zeros", () => {
    const size = 5;
    const buffer = Buffer.alloc(size);

    for (const byte of buffer) {
      assert.strictEqual(byte, 0);
    }
  });
});

describe("Buffer.from", () => {
  it("should create a buffer from a string with utf-8 encoding", () => {
    const input = "Hello, world!";
    const buffer = Buffer.from(input, "utf-8");

    assert.strictEqual(buffer.toString(), input);
  });

  it("should create a buffer from an array of bytes", () => {
    const byteArray = [65, 66, 67, 68, 69]; // ASCII values of A, B, C, D, E
    const buffer = Buffer.from(byteArray);

    for (let i = 0; i < byteArray.length; i++) {
      assert.strictEqual(buffer[i], byteArray[i]);
    }
  });

  it("should create a buffer from a string with base64 encoding", () => {
    const input = "SGVsbG8sIHdvcmxkIQ==";
    const buffer = Buffer.from(input, "base64");

    assert.strictEqual(buffer.toString(), "Hello, world!");
  });

  it("should create a buffer from a string with hex encoding", () => {
    const input = "48656c6c6f2c20776f726c6421";
    const buffer = Buffer.from(input, "hex");

    assert.strictEqual(buffer.toString(), "Hello, world!");
  });

  it("should create a buffer from a portion of an array with offset and length", () => {
    const byteArray = [65, 66, 67, 68, 69]; // ASCII values of A, B, C, D, E
    const offset = 1;
    const length = 3;

    // @ts-ignore
    const buffer = Buffer.from(byteArray, offset, length);

    assert.strictEqual(buffer.length, length);
    for (let i = 0; i < length; i++) {
      assert.strictEqual(buffer[i], byteArray[offset + i]);
    }
  });

  it("should handle offset and length overflows", () => {
    const byteArray = [65, 66, 67, 68, 69]; // ASCII values of A, B, C, D, E
    let length = 99;
    let offset = 0;
    // @ts-ignore
    let buffer = Buffer.from(byteArray, offset, length);
    assert.strictEqual(buffer.length, byteArray.length);
    for (let i = 0; i < length; i++) {
      assert.strictEqual(buffer[i], byteArray[offset + i]);
    }

    // @ts-ignore
    buffer = Buffer.from(byteArray, 99, 2);
    assert.strictEqual(buffer.length, 0);

    // @ts-ignore
    buffer = Buffer.from(byteArray, 99, 999);
    assert.strictEqual(buffer.length, 0);
  });
});

describe("toString", () => {
  it("should convert buffer to a string with utf-8 encoding", () => {
    const input = "Hello, world!";
    const buffer = Buffer.from(input);

    assert.strictEqual(buffer.toString("utf-8"), input);
  });

  it("should convert buffer to a string with base64 encoding", () => {
    const input = "SGVsbG8sIHdvcmxkIQ==";
    const buffer = Buffer.from(input, "base64");

    assert.strictEqual(buffer.toString("base64"), input);
  });

  it("should convert buffer to a string with hex encoding", () => {
    const input = "48656c6c6f2c20776f726c6421";
    const buffer = Buffer.from(input, "hex");

    assert.strictEqual(buffer.toString("hex"), input);
  });
});
