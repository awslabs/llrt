import defaultImport from "crypto";

describe("crypto object/module", () => {
  it("should have a createHash()", () => {
    expect(crypto.createHash).toBeDefined();
    expect(defaultImport.createHash).toBeDefined();
  });
  it("should have a createHmac()", () => {
    expect(crypto.createHmac).toBeDefined();
    expect(defaultImport.createHmac).toBeDefined();
  });
  it("should have a randomBytes()", () => {
    expect(crypto.randomBytes).toBeDefined();
    expect(defaultImport.randomBytes).toBeDefined();
  });
  it("should have a randomInt()", () => {
    expect(crypto.randomInt).toBeDefined();
    expect(defaultImport.randomInt).toBeDefined();
  });
  it("should have a randomUUID()", () => {
    expect(crypto.randomUUID).toBeDefined();
    expect(defaultImport.randomUUID).toBeDefined();
  });
  it("should have a randomFillSync()", () => {
    expect(crypto.randomFillSync).toBeDefined();
    expect(defaultImport.randomFillSync).toBeDefined();
  });
  it("should have a randomFill()", () => {
    expect(crypto.randomFill).toBeDefined();
    expect(defaultImport.randomFill).toBeDefined();
  });
});

describe("Hashing", () => {
  it("should hash to sha256 with b64 encoding", () => {
    let hash = defaultImport
      .createHash("sha256")
      .update("message")
      .digest("base64");
    expect(hash).toEqual("q1MKE+RZFJgrefm34/uplM/R8/si9xzqGvvwK0YMbR0=");
  });

  it("should hash to sha256 with hex encoding", () => {
    let hash = defaultImport
      .createHash("sha256")
      .update("message")
      .digest("hex");
    expect(hash).toEqual(
      "ab530a13e45914982b79f9b7e3fba994cfd1f3fb22f71cea1afbf02b460c6d1d"
    );
  });

  it("should hash to hmac-sha256 with b64 encoding", () => {
    let hash = crypto
      .createHmac("sha256", "key")
      .update("message")
      .digest("base64");
    expect(hash).toEqual("bp7ym3X//Ft6uuUn1Y/a2y/kLnIZARl2kXNDBl9Y7Uo=");
  });

  it("should hash to hmac-sha256 with hex encoding", () => {
    let hash = crypto
      .createHmac("sha256", "key")
      .update("message")
      .digest("hex");
    expect(hash).toEqual(
      "6e9ef29b75fffc5b7abae527d58fdadb2fe42e7219011976917343065f58ed4a"
    );
  });
});

describe("random", () => {
  it("should generate a random buffer synchronously using randomFillSync", () => {
    const buffer = defaultImport.randomFillSync(Buffer.alloc(16));
    expect(buffer.length).toEqual(16);
  });

  it("should generate a random buffer asynchronously using randomFill", (done) => {
    defaultImport.randomFill(Buffer.alloc(16), (err, buffer) => {
      expect(err).toBeNull();
      expect(buffer.length).toEqual(16);
      done();
    });
  });

  it("should generate random bytes synchronously into a Uint8Array using randomFillSync", () => {
    const uint8Array = new Uint8Array(16);
    defaultImport.randomFillSync(uint8Array);
    expect(uint8Array.length).toEqual(16);
    for (const byte of uint8Array) {
      expect(byte >= 0 && byte <= 255).toBeTruthy();
    }
  });

  it("should generate random bytes asynchronously into a DataView using randomFill", (done) => {
    const dataView = new DataView(new ArrayBuffer(32));
    defaultImport.randomFill(dataView, (err, buffer) => {
      expect(err).toBeNull();
      expect(buffer.buffer).toEqual(dataView.buffer);
      expect(dataView.byteLength).toEqual(32);
      for (let i = 0; i < 32; i++) {
        expect(
          dataView.getUint8(i) >= 0 && dataView.getUint8(i) <= 255
        ).toBeTruthy();
      }
      done();
    });
  });

  it("should generate a random UUID using randomUUID", () => {
    const uuid = defaultImport.randomUUID();
    expect(uuid.length).toEqual(36);
    const uuidRegex =
      /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;
    expect(uuid).toMatch(uuidRegex);
  });

  it("should generate a random bytes buffer using randomBytes", () => {
    const buffer = defaultImport.randomBytes(16);
    expect(buffer).toBeInstanceOf(Buffer);
    expect(buffer.length).toEqual(16);
  });

  it("should generate a random int using randomInt", () => {
    // Do it 10 times, to make sure we respect min and max
    for (const number of [...Array(10).keys()]) {
      const randomInt = defaultImport.randomInt(
        Number.MAX_SAFE_INTEGER - 1,
        Number.MAX_SAFE_INTEGER
      );
      expect(typeof randomInt).toEqual("number");
      expect(Number.MAX_SAFE_INTEGER - 1).toEqual(randomInt);
      expect(typeof randomInt).toEqual("number");
    }

    // Do it 20 times to make sure we never get values outside the range
    for (const number of [...Array(20).keys()]) {
      const randomInt = defaultImport.randomInt(0, 5);
      expect(randomInt).toBeLessThan(5);
      expect(randomInt).toBeGreaterThanOrEqual(0);
    }
  });
});
