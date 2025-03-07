// Test static methods
describe("Buffer.alloc", () => {
  it("should create a buffer with specified size and fill with zeros (default fill)", () => {
    const size = 10;
    const buffer = Buffer.alloc(size);

    expect(buffer.length).toEqual(size);

    for (const byte of buffer) {
      expect(byte).toEqual(0);
    }
  });

  it("should create a buffer with specified size and fill with a string value", () => {
    const size = 8;
    const fillString = "abc";
    const buffer = Buffer.alloc(size, fillString);

    expect(buffer.toString()).toEqual("abcabcab");
  });

  it("should create a buffer with specified size and fill with an encoded string value", () => {
    const size = 8;
    const fillString = "616263";
    const buffer = Buffer.alloc(size, fillString, "hex");

    expect(buffer.toString()).toEqual("abcabcab");
  });

  it("should create a buffer with specified size and fill with a Buffer value", () => {
    const size = 6;
    const fillBuffer = Buffer.from([1, 2, 3]);
    const buffer = Buffer.alloc(size, fillBuffer);

    expect(buffer).toStrictEqual(Buffer.from([1, 2, 3, 1, 2, 3]));
  });

  it("should create a buffer with specified size and fill with a Uint8Array value", () => {
    const size = 5;
    const fillUint8Array = new Uint8Array([5, 10, 15]);
    const buffer = Buffer.alloc(size, fillUint8Array);

    expect(buffer).toStrictEqual(Buffer.from([5, 10, 15, 5, 10]));
  });

  it("should create a buffer with specified size and fill with an integer value", () => {
    const size = 4;
    const fillInteger = 42;
    const buffer = Buffer.alloc(size, fillInteger);

    for (const byte of buffer) {
      expect(byte).toEqual(fillInteger);
    }
  });

  it("should throw an error when fill argument is invalid", () => {
    const size = 10;
    let buffer = Buffer.alloc(size, true as any);
    for (const byte of buffer) {
      expect(byte).toEqual(0);
    }
  });
});

describe("Buffer.allocUnsafe", () => {
  it("should create a buffer of the specified size", () => {
    const size = 10;
    const buffer = Buffer.allocUnsafe(size);

    expect(buffer.length).toEqual(size);
    for (const byte of buffer) {
      expect(byte).toBeDefined();
    }
  });

  it("should create an empty buffer when size is 0", () => {
    const size = 0;
    const buffer = Buffer.allocUnsafe(size);

    expect(buffer.length).toEqual(size);
  });

  it("should throw a TypeError when size is negative", () => {
    expect(() => {
      const size = -1;
      const buffer = Buffer.allocUnsafe(size);
    }).toThrow(TypeError);
  });
});

describe("Buffer.allocUnsafeSlow", () => {
  it("should create a buffer of the specified size", () => {
    const size = 10;
    const buffer = Buffer.allocUnsafeSlow(size);

    expect(buffer.length).toEqual(size);
    for (const byte of buffer) {
      expect(byte).toBeDefined();
    }
  });

  it("should create an empty buffer when size is 0", () => {
    const size = 0;
    const buffer = Buffer.allocUnsafeSlow(size);

    expect(buffer.length).toEqual(size);
  });

  it("should throw a TypeError when size is negative", () => {
    expect(() => {
      const size = -1;
      const buffer = Buffer.allocUnsafeSlow(size);
    }).toThrow(TypeError);
  });
});

describe("Buffer.byteLength", () => {
  it("should return the correct byte length for ASCII string", () => {
    const length = Buffer.byteLength("Hello");

    expect(length).toEqual(5);
  });

  it("should return the correct byte length for UTF-8 string", () => {
    const length = Buffer.byteLength("👋");

    expect(length).toEqual(4);
  });

  it("should return the correct byte length for UTF-8 string", () => {
    const length = Buffer.byteLength("你好");

    expect(length).toEqual(6);
  });

  it("should return the correct byte length for a buffer", () => {
    const buffer = Buffer.from([1, 2, 3, 4, 5]);
    const length = Buffer.byteLength(buffer);

    expect(length).toEqual(5);
  });

  it("should return the correct byte length for a hex-encoded string", () => {
    const length = Buffer.byteLength("deadbeef", "hex");

    expect(length).toEqual(4);
  });

  it("should return the correct byte length for a base64-encoded string", () => {
    const length = Buffer.byteLength("SGVsbG8gV29ybGQ=", "base64");

    expect(length).toEqual(11);
  });
});

describe("Buffer.concat", () => {
  it("should concatenate buffers", () => {
    const buffer1 = Buffer.from("Hello");
    const buffer2 = Buffer.from(" ");
    const buffer3 = Buffer.from("World");
    const resultBuffer = Buffer.concat([buffer1, buffer2, buffer3]);

    expect(resultBuffer.toString()).toEqual("Hello World");
  });

  it("should handle empty buffers in the array", () => {
    const buffer1 = Buffer.from("Hello");
    const buffer2 = Buffer.from("");
    const buffer3 = Buffer.from("World");
    const resultBuffer = Buffer.concat([buffer1, buffer2, buffer3]);

    expect(resultBuffer.toString()).toEqual("HelloWorld");
  });

  it("should handle an array with a single buffer", () => {
    const buffer = Buffer.from("SingleBuffer");
    const resultBuffer = Buffer.concat([buffer]);

    expect(resultBuffer.toString()).toEqual("SingleBuffer");
  });

  it("should handle an empty array of buffers", () => {
    const resultBuffer = Buffer.concat([]);

    expect(resultBuffer.toString()).toEqual("");
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

    expect(resultBuffer.toString()).toEqual("1234");

    const resultBuffer2 = Buffer.concat([buffer1, buffer2, buffer3], 3);

    expect(resultBuffer2.toString()).toEqual("123");
  });

  it("should throw an error when totalLength is less than the actual length of concatenated buffers", () => {
    const buffer1 = Buffer.from("Hello");
    const buffer2 = Buffer.from("World");
    const resultBuffer = Buffer.concat([buffer1, buffer2], 999);

    expect(resultBuffer.toString()).toEqual("HelloWorld");
    expect(resultBuffer.length).toEqual(buffer1.length + buffer2.length);
  });
});

describe("Buffer.from", () => {
  it("should create a buffer from a string with utf-8 encoding", () => {
    const input = "Hello, world!";
    const buffer = Buffer.from(input, "utf-8");

    expect(buffer.toString()).toEqual(input);
  });

  it("should create a buffer from an array of bytes", () => {
    const byteArray = [65, 66, 67, 68, 69]; // ASCII values of A, B, C, D, E
    const buffer = Buffer.from(byteArray);

    for (let i = 0; i < byteArray.length; i++) {
      expect(buffer[i]).toEqual(byteArray[i]);
    }
  });

  it("should create a buffer from a string with base64 encoding", () => {
    const input = "SGVsbG8sIHdvcmxkIQ==";
    const buffer = Buffer.from(input, "base64");
    expect(buffer.toString()).toEqual("Hello, world!");

    const input2 = "SGVsbG8sIHdvcmxkIQ";
    const buffer2 = Buffer.from(input2, "base64");
    expect(buffer2.toString()).toEqual("Hello, world!");
  });

  it("should create a buffer from a string with base64 encoding that contains / or +", () => {
    const input = "PD8+MTIz";
    const buffer = Buffer.from(input, "base64");
    expect(buffer.toString()).toEqual("<?>123");

    const input3 = "PD8/PjEyMw==";
    const buffer3 = Buffer.from(input3, "base64");
    expect(buffer3.toString()).toEqual("<??>123");
  });

  // https://en.wikipedia.org/wiki/Base64#URL_applications
  it("should create a buffer from a string with URL safe base64 encoding that contains _ or -", () => {
    const input = "PD8-MTIz";
    const buffer = Buffer.from(input, "base64");
    expect(buffer.toString()).toEqual("<?>123");

    const input3 = "PD8_PjEyMw";
    const buffer3 = Buffer.from(input3, "base64");
    expect(buffer3.toString()).toEqual("<??>123");
  });

  it("should create a buffer from a string with hex encoding", () => {
    const input = "48656c6c6f2c20776f726c6421";
    const buffer = Buffer.from(input, "hex");

    expect(buffer.toString()).toEqual("Hello, world!");
  });

  it("should create a buffer from a portion of an array with offset and length", () => {
    const byteArray = [65, 66, 67, 68, 69]; // ASCII values of A, B, C, D, E
    const offset = 1;
    const length = 3;

    // @ts-ignore
    const buffer = Buffer.from(byteArray, offset, length);

    expect(buffer.length).toEqual(length);
    for (let i = 0; i < length; i++) {
      expect(buffer[i]).toEqual(byteArray[offset + i]);
    }
  });

  it("should handle offset and length overflows", () => {
    const byteArray = [65, 66, 67, 68, 69]; // ASCII values of A, B, C, D, E
    let length = 99;
    let offset = 0;
    // @ts-ignore
    let buffer = Buffer.from(byteArray, offset, length);
    expect(buffer.length).toEqual(byteArray.length);
    for (let i = 0; i < length; i++) {
      expect(buffer[i]).toEqual(byteArray[offset + i]);
    }

    // @ts-ignore
    buffer = Buffer.from(byteArray, 99, 2);
    expect(buffer.length).toEqual(0);

    // @ts-ignore
    buffer = Buffer.from(byteArray, 99, 999);
    expect(buffer.length).toEqual(0);
  });

  it("should use same memory for sub arrays", () => {
    const typedArray = new Uint8Array([65, 66, 67, 68, 69]);

    const a = Buffer.from(typedArray.buffer);
    const b = Buffer.from(typedArray.subarray(1, 4));
    const c = Buffer.from(a);

    expect(a.buffer).toStrictEqual(b.buffer);
    expect(a.toString()).toEqual("ABCDE");
    expect(b.toString()).toEqual("BCD");
    expect(c.toString()).toEqual("ABCDE");

    typedArray.set([70, 71], 1);

    expect(a.toString()).toEqual("AFGDE");
    expect(b.toString()).toEqual("FGD");
    expect(c.toString()).toEqual("ABCDE");
  });
});

describe("Buffer.isBuffer", () => {
  it("should return true when the object being tested is an instance of Buffer", () => {
    const buffer = Buffer.from("Hello, world!");

    expect(Buffer.isBuffer(buffer)).toEqual(true);
  });

  it("should return false when the object being tested is not an instance of Buffer", () => {
    expect(Buffer.isBuffer(false)).toEqual(false);
    expect(Buffer.isBuffer(undefined)).toEqual(false);
    expect(Buffer.isBuffer(null)).toEqual(false);
    expect(Buffer.isBuffer("Buffer")).toEqual(false);
    expect(Buffer.isBuffer(Buffer)).toEqual(false);
  });
});

describe("Buffer.isEncoding", () => {
  it("should return true when input is a valid encoding name", () => {
    expect(Buffer.isEncoding("utf8")).toEqual(true);
    expect(Buffer.isEncoding("hex")).toEqual(true);
    expect(Buffer.isEncoding("base64")).toEqual(true);
  });

  it("should return false when input is not a valid encoding name", () => {
    expect(Buffer.isEncoding(false as unknown as string)).toEqual(false);
    expect(Buffer.isEncoding(undefined as unknown as string)).toEqual(false);
    expect(Buffer.isEncoding(null as unknown as string)).toEqual(false);
    expect(Buffer.isEncoding("utf8/8")).toEqual(false);
  });
});

// Test prototype methods
describe("copy", () => {
  it("should copy the entire source buffer to the destination buffer", () => {
    const bufSrc = Buffer.from("abcdefghijklmnopqrstuvwxyz");
    const bufDest = Buffer.from("**************************");
    expect(bufSrc.copy(bufDest)).toEqual(26);
    expect(bufDest.toString()).toEqual("abcdefghijklmnopqrstuvwxyz");
  });

  it("should copy the entire source buffer starting from a specified offset in the destination buffer", () => {
    const bufSrc = Buffer.from("abcdefghijklmnopqrstuvwxyz");
    const bufDest = Buffer.from("**************************");
    expect(bufSrc.copy(bufDest, 5)).toEqual(21);
    expect(bufDest.toString()).toEqual("*****abcdefghijklmnopqrstu");
  });

  it("should copy a portion of the source buffer starting from a specified source offset to the destination buffer at a specified offset", () => {
    const bufSrc = Buffer.from("abcdefghijklmnopqrstuvwxyz");
    const bufDest = Buffer.from("**************************");
    expect(bufSrc.copy(bufDest, 5, 10)).toEqual(16);
    expect(bufDest.toString()).toEqual("*****klmnopqrstuvwxyz*****");
  });

  it("should copy a specific range of the source buffer to the destination buffer at a specified offset", () => {
    const bufSrc = Buffer.from("abcdefghijklmnopqrstuvwxyz");
    const bufDest = Buffer.from("**************************");
    expect(bufSrc.copy(bufDest, 5, 10, 15)).toEqual(5);
    expect(bufDest.toString()).toEqual("*****klmno****************");
  });

  it("should return 0 and not modify the destination buffer when the source start index is greater than the source end index", () => {
    const bufSrc = Buffer.from("abcdefghijklmnopqrstuvwxyz");
    const bufDest = Buffer.from("**************************");
    expect(bufSrc.copy(bufDest, 5, 10, 9)).toEqual(0);
    expect(bufDest.toString()).toEqual("**************************");
  });
});

describe("subarray", () => {
  it("should create a subarray from a buffer with the specified start and end indices", () => {
    const buffer = Buffer.from("Hello, world!");
    const subBuffer = buffer.subarray(7, 12);

    expect(subBuffer.toString()).toEqual("world");
  });

  it("should return a subarray from the start index to the end of the buffer when end index is omitted", () => {
    const buffer = Buffer.from("Hello, world!");
    const subBuffer = buffer.subarray(7);

    expect(subBuffer.toString()).toEqual("world!");
  });

  it("should return an empty buffer when the start index equals the end index", () => {
    const buffer = Buffer.from("Hello, world!");
    const subBuffer = buffer.subarray(5, 5);

    expect(subBuffer.length).toEqual(0);
    expect(subBuffer.toString()).toEqual("");
  });

  it("should create a subarray with the same content as the original buffer when start and end indices cover the entire buffer", () => {
    const buffer = Buffer.from("Hello, world!");
    const subBuffer = buffer.subarray(0, buffer.length);

    expect(subBuffer.toString()).toEqual("Hello, world!");
    expect(subBuffer).not.toBe(buffer); // Should be a new buffer, not the original one
  });

  it("should handle negative start and end indices", () => {
    const buffer = Buffer.from("Hello, world!");
    const subBuffer = buffer.subarray(-6, -1);

    expect(subBuffer.toString()).toEqual("world");
  });

  it("should handle out-of-bounds start and end indices", () => {
    const buffer = Buffer.from("Hello, world!");

    const subBuffer1 = buffer.subarray(-100, 5);
    expect(subBuffer1.toString()).toEqual("Hello");

    const subBuffer2 = buffer.subarray(0, 100);
    expect(subBuffer2.toString()).toEqual("Hello, world!");

    const subBuffer3 = buffer.subarray(50, 100);
    expect(subBuffer3.length).toEqual(0);
  });

  it("should share memory with the original buffer", () => {
    const buffer = Buffer.from("Hello, world!");
    const subBuffer = buffer.subarray(0, 5);

    const lowerCaseH = "h".charCodeAt(0);
    subBuffer[0] = lowerCaseH;
    expect(buffer[0]).toEqual(lowerCaseH);
    expect(subBuffer.toString()).toEqual("hello");
  });

  it("should not throw errors when start and end are out of order, but should return an empty buffer", () => {
    const buffer = Buffer.from("Hello, world!");
    const subBuffer = buffer.subarray(10, 5);

    expect(subBuffer.length).toEqual(0);
    expect(subBuffer.toString()).toEqual("");
  });
});

describe("toString", () => {
  it("should convert buffer to a string with utf-8 encoding", () => {
    const input = "Hello, world!";
    const buffer = Buffer.from(input);

    expect(buffer.toString("utf-8")).toEqual(input);
  });

  it("should convert buffer to a string with base64 encoding", () => {
    const input = "SGVsbG8sIHdvcmxkIQ==";
    const buffer = Buffer.from(input, "base64");

    expect(buffer.toString("base64")).toEqual(input);
  });

  it("should convert buffer to a string with hex encoding", () => {
    const input = "48656c6c6f2c20776f726c6421";
    const buffer = Buffer.from(input, "hex");

    expect(buffer.toString("hex")).toEqual(input);
  });
});

describe("write", () => {
  it("should write a UTF-8 string into a buffer and return the correct byte length", () => {
    const buf1 = Buffer.alloc(15);
    expect(buf1.write("こんにちは", "utf-8")).toEqual(15); // "こんにちは" means 'hello' in japanese.
    expect(buf1.toString("utf8")).toEqual("こんにちは");
  });

  it("should write a hex string into a buffer and correctly convert it to UTF-8", () => {
    const buf2 = Buffer.alloc(15);
    expect(buf2.write("68656c6c6f", "hex")).toEqual(5); // 68656c6c6f -> 'hello'
    expect(buf2.toString("utf8").substring(0, 5)).toEqual("hello");
  });

  it("should write a UTF-8 string into a buffer with an explicit offset of 0", () => {
    const buf1 = Buffer.alloc(15);
    expect(buf1.write("こんにちは", 0, "utf-8")).toEqual(15);
    expect(buf1.toString("utf8")).toEqual("こんにちは");
  });

  it("should write a hex string into a buffer with an explicit offset of 0", () => {
    const buf2 = Buffer.alloc(15);
    expect(buf2.write("68656c6c6f", 0, "hex")).toEqual(5);
    expect(buf2.toString("utf8").substring(0, 5)).toEqual("hello");
  });

  it("should write a UTF-8 string at offset 12 and store only part of it", () => {
    const buf1 = Buffer.alloc(15);
    expect(buf1.write("こんにちは", 12, "utf-8")).toEqual(3);
    expect(buf1.toString("utf8").substring(12)).toEqual("こ");
  });

  it("should write a hex string at offset 12 and store only part of it", () => {
    const buf2 = Buffer.alloc(15);
    expect(buf2.write("68656c6c6f", 12, "hex")).toEqual(3);
    expect(buf2.toString("utf8").substring(12)).toEqual("hel");
  });

  it("should write only the first 3 bytes of a UTF-8 string and store a partial character", () => {
    const buf1 = Buffer.alloc(15);
    expect(buf1.write("こんにちは", 0, 3, "utf-8")).toEqual(3);
    expect(buf1.toString("utf8").substring(0, 1)).toEqual("こ"); // Returning characters instead of bytes
  });

  it("should write only the first 3 bytes of a hex string and correctly store the data", () => {
    const buf2 = Buffer.alloc(15);
    expect(buf2.write("68656c6c6f", 0, 3, "hex")).toEqual(3);
    expect(buf2.toString("utf8").substring(0, 3)).toEqual("hel");
  });

  it("should write a UTF-8 string at offset 9 with a length of 12 bytes and store part of it", () => {
    const buf1 = Buffer.alloc(15);
    expect(buf1.write("こんにちは", 9, 12, "utf-8")).toEqual(6);
    expect(buf1.toString("utf8").substring(9, 12)).toEqual("こん");
  });

  it("should write a hex string at offset 9 with a length of 12 bytes and store part of it", () => {
    const buf2 = Buffer.alloc(15);
    expect(buf2.write("68656c6c6f", 9, 12, "hex")).toEqual(5);
    expect(buf2.toString("utf8").substring(9, 12)).toEqual("hel");
  });
});

describe("writeInt32BE", () => {
  it("should write a 32-bit integer in big-endian format at the beginning of the buffer", () => {
    const buf = Buffer.alloc(8);
    expect(buf.writeInt32BE(0x01020304)).toEqual(4);
    expect(buf).toEqual(Buffer.from([1, 2, 3, 4, 0, 0, 0, 0]));
  });

  it("should write a 32-bit integer in big-endian format at the specified offset in the buffer", () => {
    const buf = Buffer.alloc(8);
    expect(buf.writeInt32BE(0x01020304, 4)).toEqual(8);
    expect(buf).toEqual(Buffer.from([0, 0, 0, 0, 1, 2, 3, 4]));
  });

  it("should throw a RangeError if the offset is out of bounds", () => {
    expect(() => {
      const buf = Buffer.alloc(8);
      buf.writeInt32BE(0x01020304, 5);
    }).toThrow(RangeError);
  });
});
