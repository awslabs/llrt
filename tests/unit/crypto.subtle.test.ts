describe("SubtleCrypto digest", () => {
  const encoder = new TextEncoder();
  const data = encoder.encode("This is test message.");

  it("should calculate correctly SHA-1/256/384/512 digest", () => {
    [
      {
        name: "SHA-1",
        digest: new Uint8Array([
          77, 178, 99, 24, 75, 24, 35, 67, 75, 116, 194, 145, 251, 77, 201, 158,
          163, 128, 52, 146,
        ]),
      },
      {
        name: "SHA-256",
        digest: new Uint8Array([
          25, 251, 197, 98, 46, 227, 66, 238, 69, 151, 67, 175, 68, 184, 76,
          182, 55, 172, 65, 183, 49, 68, 7, 196, 44, 100, 140, 80, 173, 34, 85,
          162,
        ]),
      },
      {
        name: "SHA-384",
        digest: new Uint8Array([
          120, 197, 163, 25, 168, 231, 230, 188, 103, 31, 71, 249, 169, 230,
          153, 177, 79, 131, 234, 88, 93, 74, 22, 71, 169, 225, 35, 40, 129,
          238, 168, 107, 148, 105, 238, 23, 160, 190, 147, 195, 162, 135, 202,
          230, 26, 130, 124, 245,
        ]),
      },
      {
        name: "SHA-512",
        digest: new Uint8Array([
          197, 205, 134, 50, 153, 136, 53, 103, 159, 209, 236, 27, 212, 147,
          229, 162, 64, 1, 116, 206, 59, 187, 12, 223, 135, 121, 147, 143, 26,
          203, 161, 238, 57, 27, 254, 202, 96, 207, 172, 168, 12, 47, 150, 164,
          182, 235, 183, 159, 134, 226, 198, 183, 61, 128, 211, 133, 33, 12,
          168, 19, 139, 120, 80, 145,
        ]),
      },
    ].forEach(async function (t) {
      const result = new Uint8Array(await crypto.subtle.digest(t.name, data));
      expect(result).toEqual(t.digest);
    });
  });
});

describe("SubtleCrypto generateKey", () => {
  const extractable = false;
  const keyUsage = ["sign", "verify"];

  it("should be processing AES-CBC/AES-CTR/AES-GCM/AES-KW algorithm", () => {
    [
      { name: "AES-CBC", length: 128 },
      { name: "AES-CBC", length: 192 },
      { name: "AES-CBC", length: 256 },
      { name: "AES-CTR", length: 128 },
      { name: "AES-CTR", length: 192 },
      { name: "AES-CTR", length: 256 },
      { name: "AES-GCM", length: 128 },
      { name: "AES-GCM", length: 192 },
      { name: "AES-GCM", length: 256 },
      { name: "AES-KW", length: 128 },
      { name: "AES-KW", length: 192 },
      { name: "AES-KW", length: 256 },
    ].forEach(async function (t) {
      const algorithm = { name: t.name, namedCurve: t.length };

      const key = await crypto.subtle.generateKey(
        algorithm,
        extractable,
        keyUsage
      );

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect(key.algorithm.length).toEqual(algorithm.length);
      expect(key.extractable).toEqual(extractable);
    });
  });

  it("should be processing HMAC algorithm", () => {
    [
      { name: "HMAC", hash: "SHA-1" },
      { name: "HMAC", hash: "SHA-256" },
      { name: "HMAC", hash: "SHA-384" },
      { name: "HMAC", hash: "SHA-512" },
    ].forEach(async function (t) {
      const algorithm = { name: t.name, hash: t.hash };

      const key = await crypto.subtle.generateKey(
        algorithm,
        extractable,
        keyUsage
      );

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect(key.algorithm.hash).toEqual(algorithm.hash);
      expect(key.extractable).toEqual(extractable);
    });
  });

  it("should be processing ECDH/ECDSA algorithm", () => {
    [
      { name: "ECDH", namedCurve: "P-256" },
      { name: "ECDH", namedCurve: "P-384" },
      { name: "ECDSA", namedCurve: "P-256" },
      { name: "ECDSA", namedCurve: "P-384" },
    ].forEach(async function (t) {
      const algorithm = { name: t.name, namedCurve: t.namedCurve };

      const { privateKey, publicKey } = await crypto.subtle.generateKey(
        algorithm,
        extractable,
        keyUsage
      );

      expect(privateKey.algorithm.name).toEqual(algorithm.name);
      expect(privateKey.algorithm.namedCurve).toEqual(algorithm.namedCurve);
      expect(privateKey.extractable).toEqual(extractable);

      expect(publicKey.algorithm.name).toEqual(algorithm.name);
      expect(publicKey.algorithm.namedCurve).toEqual(algorithm.namedCurve);
      expect(publicKey.extractable).toEqual(true);
    });
  });

  it("should be processing RSA-PSS/RSA-OAEP/RSASSA-PKCS1-v1_5 algorithm", () => {
    [
      {
        name: "RSA-PSS",
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-256",
      },
      {
        name: "RSA-PSS",
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-384",
      },
      {
        name: "RSA-PSS",
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-512",
      },
      {
        name: "RSA-OAEP",
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-256",
      },
      {
        name: "RSA-OAEP",
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-384",
      },
      {
        name: "RSA-OAEP",
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-512",
      },
      {
        name: "RSASSA-PKCS1-v1_5",
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-256",
      },
      {
        name: "RSASSA-PKCS1-v1_5",
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-384",
      },
      {
        name: "RSASSA-PKCS1-v1_5",
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-512",
      },
    ].forEach(async function (t) {
      const algorithm = {
        name: t.name,
        publicExponent: t.publicExponent,
        hash: t.hash,
      };

      const { privateKey, publicKey } = await crypto.subtle.generateKey(
        algorithm,
        extractable,
        keyUsage
      );

      expect(privateKey.algorithm.name).toEqual(algorithm.name);
      expect(privateKey.algorithm.publicExponent).toEqual(
        algorithm.publicExponent
      );
      expect(privateKey.algorithm.hash).toEqual(algorithm.hash);
      expect(privateKey.extractable).toEqual(extractable);

      expect(publicKey.algorithm.name).toEqual(algorithm.name);
      expect(publicKey.algorithm.publicExponent).toEqual(
        algorithm.publicExponent
      );
      expect(publicKey.algorithm.hash).toEqual(algorithm.hash);
      expect(publicKey.extractable).toEqual(true);
    });
  });
});
