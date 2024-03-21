describe("Buffer.alloc", () => {
  it("should create a buffer with specified size and fill with zeros (default fill)", () => {
    const size = 10;
    const buffer = Buffer.alloc(size);

    expect(buffer.length).toEqual(size)

    for (const byte of buffer) {
      expect(byte).toEqual(0)
    }
  });

  it("should create a buffer with specified size and fill with a string value", () => {
    const size = 8;
    const fillString = "abc";
    const buffer = Buffer.alloc(size, fillString);

    expect(buffer.toString()).toEqual("abcabcab")
  });

  it("should create a buffer with specified size and fill with an encoded string value", () => {
    const size = 8;
    const fillString = "616263";
    const buffer = Buffer.alloc(size, fillString, "hex");

    expect(buffer.toString()).toEqual("abcabcab")
  });

  it("should create a buffer with specified size and fill with a Buffer value", () => {
    const size = 6;
    const fillBuffer = Buffer.from([1, 2, 3]);
    const buffer = Buffer.alloc(size, fillBuffer);

    expect(buffer).toStrictEqual(Buffer.from([1, 2, 3, 1, 2, 3]))
  });

  it("should create a buffer with specified size and fill with a Uint8Array value", () => {
    const size = 5;
    const fillUint8Array = new Uint8Array([5, 10, 15]);
    const buffer = Buffer.alloc(size, fillUint8Array);

    expect(buffer).toStrictEqual(Buffer.from([5, 10, 15, 5, 10]))
  });

  it("should create a buffer with specified size and fill with an integer value", () => {
    const size = 4;
    const fillInteger = 42;
    const buffer = Buffer.alloc(size, fillInteger);

    for (const byte of buffer) {
      expect(byte).toEqual(fillInteger)
    }
  });

  it("should throw an error when fill argument is invalid", () => {
    const size = 10;
    let buffer = Buffer.alloc(size, true as any);
    for (const byte of buffer) {
      expect(byte).toEqual(0)
    }
  });
});

describe("Buffer.from", () => {
  it("should create a buffer from a string with utf-8 encoding", () => {
    const input = "Hello, world!";
    const buffer = Buffer.from(input, "utf-8");

    expect(buffer.toString()).toEqual(input)
  });

  it("should create a buffer from an array of bytes", () => {
    const byteArray = [65, 66, 67, 68, 69]; // ASCII values of A, B, C, D, E
    const buffer = Buffer.from(byteArray);

    for (let i = 0; i < byteArray.length; i++) {
      expect(buffer[i]).toEqual(byteArray[i])
    }
  });

  it("should create a buffer from a string with base64 encoding", () => {
    const input = "SGVsbG8sIHdvcmxkIQ==";
    const buffer = Buffer.from(input, "base64");

    expect(buffer.toString()).toEqual("Hello, world!")

    const input2 = "SGVsbG8sIHdvcmxkIQ";
    const buffer2 = Buffer.from(input2, "base64");
    expect(buffer2.toString()).toEqual("Hello, world!")
  });

  it("should create a buffer from a string with hex encoding", () => {
    const input = "48656c6c6f2c20776f726c6421";
    const buffer = Buffer.from(input, "hex");

    expect(buffer.toString()).toEqual("Hello, world!")
  });

  it("should create a buffer from a portion of an array with offset and length", () => {
    const byteArray = [65, 66, 67, 68, 69]; // ASCII values of A, B, C, D, E
    const offset = 1;
    const length = 3;

    // @ts-ignore
    const buffer = Buffer.from(byteArray, offset, length);

    expect(buffer.length).toEqual(length)
    for (let i = 0; i < length; i++) {
      expect(buffer[i]).toEqual(byteArray[offset + i])
    }
  });

  it("should handle offset and length overflows", () => {
    const byteArray = [65, 66, 67, 68, 69]; // ASCII values of A, B, C, D, E
    let length = 99;
    let offset = 0;
    // @ts-ignore
    let buffer = Buffer.from(byteArray, offset, length);
    expect(buffer.length).toEqual(byteArray.length)
    for (let i = 0; i < length; i++) {
      expect(buffer[i]).toEqual(byteArray[offset + i])
    }

    // @ts-ignore
    buffer = Buffer.from(byteArray, 99, 2);
    expect(buffer.length).toEqual(0)

    // @ts-ignore
    buffer = Buffer.from(byteArray, 99, 999);
    expect(buffer.length).toEqual(0)
  });
});

describe("toString", () => {
  it("should convert buffer to a string with utf-8 encoding", () => {
    const input = "Hello, world!";
    const buffer = Buffer.from(input);

    expect(buffer.toString("utf-8")).toEqual(input)
  });

  it("should convert buffer to a string with base64 encoding", () => {
    const input = "SGVsbG8sIHdvcmxkIQ==";
    const buffer = Buffer.from(input, "base64");

    expect(buffer.toString("base64")).toEqual(input)
  });

  it("should convert buffer to a string with hex encoding", () => {
    const input = "48656c6c6f2c20776f726c6421";
    const buffer = Buffer.from(input, "hex");

    expect(buffer.toString("hex")).toEqual(input)
  });
});

describe("Buffer.concat", () => {
  it("should concatenate buffers", () => {
    const buffer1 = Buffer.from("Hello");
    const buffer2 = Buffer.from(" ");
    const buffer3 = Buffer.from("World");
    const resultBuffer = Buffer.concat([buffer1, buffer2, buffer3]);

    expect(resultBuffer.toString()).toEqual("Hello World")
  });

  it("should handle empty buffers in the array", () => {
    const buffer1 = Buffer.from("Hello");
    const buffer2 = Buffer.from("");
    const buffer3 = Buffer.from("World");
    const resultBuffer = Buffer.concat([buffer1, buffer2, buffer3]);

    expect(resultBuffer.toString()).toEqual("HelloWorld")
  });

  it("should handle an array with a single buffer", () => {
    const buffer = Buffer.from("SingleBuffer");
    const resultBuffer = Buffer.concat([buffer]);

    expect(resultBuffer.toString()).toEqual("SingleBuffer")
  });

  it("should handle an empty array of buffers", () => {
    const resultBuffer = Buffer.concat([]);

    expect(resultBuffer.toString()).toEqual("")
  });

  it("should throw an error when the list contains a non-buffer", () => {
    expect(() => {
      const buffer1 = Buffer.from("Hello");
      const invalidBuffer = "InvalidBuffer";
      Buffer.concat([buffer1, invalidBuffer as any]);
    }).toThrow(TypeError);
  });

  it("should throw an error when the totalLength is too large", () => {
    expect(() => {
      const buffer1 = Buffer.from("Hello");
      const buffer2 = Buffer.alloc(2 ** 32); // 1 GB buffer
      Buffer.concat([buffer1, buffer2], 2 ** 33); // totalLength exceeding maximum allowed
    }).toThrow(RangeError);
  });

  it("should concatenate buffers with specified totalLength", () => {
    const buffer1 = Buffer.from("123");
    const buffer2 = Buffer.from("4567");
    const buffer3 = Buffer.from("89");
    const resultBuffer = Buffer.concat([buffer1, buffer2, buffer3], 4);

    expect(resultBuffer.toString()).toEqual("1234")

    const resultBuffer2 = Buffer.concat([buffer1, buffer2, buffer3], 3);

    expect(resultBuffer2.toString()).toEqual("123")
  });

  it("should throw an error when totalLength is less than the actual length of concatenated buffers", () => {
    const buffer1 = Buffer.from("Hello");
    const buffer2 = Buffer.from("World");
    const resultBuffer = Buffer.concat([buffer1, buffer2], 999);

    expect(resultBuffer.toString()).toEqual("HelloWorld")
    expect(resultBuffer.length).toEqual(buffer1.length + buffer2.length)
  });
});

describe("Buffer.byteLength", () => {
  it("should return the correct byte length for ASCII string", () => {
    const length = Buffer.byteLength("Hello");

    expect(length).toEqual(5)
  });

  it("should return the correct byte length for UTF-8 string", () => {
    const length = Buffer.byteLength("👋");

    expect(length).toEqual(4)
  });

  it("should return the correct byte length for UTF-8 string", () => {
    const length = Buffer.byteLength("你好");

    expect(length).toEqual(6)
  });

  it("should return the correct byte length for a buffer", () => {
    const buffer = Buffer.from([1, 2, 3, 4, 5]);
    const length = Buffer.byteLength(buffer);

    expect(length).toEqual(5)
  });

  it("should return the correct byte length for a hex-encoded string", () => {
    const length = Buffer.byteLength("deadbeef", "hex");

    expect(length).toEqual(4)
  });

  it("should return the correct byte length for a base64-encoded string", () => {
    const length = Buffer.byteLength("SGVsbG8gV29ybGQ=", "base64");

    expect(length).toEqual(11)
  });
});
