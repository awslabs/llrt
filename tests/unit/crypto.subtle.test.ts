import type { webcrypto } from "crypto";

const DECODER = new TextDecoder();
const ENCODER = new TextEncoder();
const TEST_MESSAGE = "This is test message.";
const ENCODED_DATA = ENCODER.encode(TEST_MESSAGE);

describe("SubtleCrypto digest", () => {
  it("should calculate correctly SHA-1/256/384/512 digest", async () => {
    const parameters: [string, number[]][] = [
      [
        "SHA-1",
        [
          77, 178, 99, 24, 75, 24, 35, 67, 75, 116, 194, 145, 251, 77, 201, 158,
          163, 128, 52, 146,
        ],
      ],
      [
        "SHA-256",
        [
          25, 251, 197, 98, 46, 227, 66, 238, 69, 151, 67, 175, 68, 184, 76,
          182, 55, 172, 65, 183, 49, 68, 7, 196, 44, 100, 140, 80, 173, 34, 85,
          162,
        ],
      ],
      [
        "SHA-384",
        [
          120, 197, 163, 25, 168, 231, 230, 188, 103, 31, 71, 249, 169, 230,
          153, 177, 79, 131, 234, 88, 93, 74, 22, 71, 169, 225, 35, 40, 129,
          238, 168, 107, 148, 105, 238, 23, 160, 190, 147, 195, 162, 135, 202,
          230, 26, 130, 124, 245,
        ],
      ],
      [
        "SHA-512",
        [
          197, 205, 134, 50, 153, 136, 53, 103, 159, 209, 236, 27, 212, 147,
          229, 162, 64, 1, 116, 206, 59, 187, 12, 223, 135, 121, 147, 143, 26,
          203, 161, 238, 57, 27, 254, 202, 96, 207, 172, 168, 12, 47, 150, 164,
          182, 235, 183, 159, 134, 226, 198, 183, 61, 128, 211, 133, 33, 12,
          168, 19, 139, 120, 80, 145,
        ],
      ],
    ];
    for (const [name, digest] of parameters) {
      const result = new Uint8Array(
        await crypto.subtle.digest(name, ENCODED_DATA)
      );

      expect(result).toEqual(new Uint8Array(digest));
    }
  });
});

describe("SubtleCrypto generateKey/sign/verify", () => {
  it("should be processing AES-CBC/AES-CTR/AES-GCM/AES-KW algorithm", async () => {
    const parameters = [
      {
        name: "AES-CBC",
        length: 128,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CBC",
        length: 192,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CBC",
        length: 256,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CTR",
        length: 128,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CTR",
        length: 192,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CTR",
        length: 256,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-GCM",
        length: 128,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-GCM",
        length: 192,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-GCM",
        length: 256,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      { name: "AES-KW", length: 128, usages: ["wrapKey", "unwrapKey"] },
      { name: "AES-KW", length: 192, usages: ["wrapKey", "unwrapKey"] },
      { name: "AES-KW", length: 256, usages: ["wrapKey", "unwrapKey"] },
    ];

    for (const t of parameters) {
      const algorithm = { name: t.name, length: t.length };

      const key = await crypto.subtle.generateKey(
        algorithm,
        false,
        t.usages as webcrypto.KeyUsage[]
      );

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect((key.algorithm as any).length).toEqual(algorithm.length);
      expect(key.extractable).toEqual(false);
    }
  });

  it("should be processing HMAC algorithm", async () => {
    const parameters = [
      { name: "HMAC", hash: "SHA-1", usages: ["sign", "verify"] },
      { name: "HMAC", hash: "SHA-256", usages: ["sign", "verify"] },
      { name: "HMAC", hash: "SHA-384", usages: ["sign", "verify"] },
      { name: "HMAC", hash: "SHA-512", usages: ["sign", "verify"] },
    ];

    for (const t of parameters) {
      const algorithm = { name: t.name, hash: t.hash };

      const key = (await crypto.subtle.generateKey(
        algorithm,
        false,
        t.usages as webcrypto.KeyUsage[]
      )) as webcrypto.CryptoKey;

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect((key.algorithm as any).hash).toEqual({ name: algorithm.hash });
      expect(key.extractable).toEqual(false);
    }
  });

  it("should be processing ECDH/ECDSA algorithm", async () => {
    const parameters = [
      {
        name: "ECDH",
        namedCurve: "P-256",
        usages: ["deriveKey", "deriveBits"],
      },
      {
        name: "ECDH",
        namedCurve: "P-384",
        usages: ["deriveKey", "deriveBits"],
      },
      {
        name: "ECDSA",
        namedCurve: "P-256",
        usages: ["sign", "verify"],
        hash: "SHA-256",
      },
      {
        name: "ECDSA",
        namedCurve: "P-384",
        usages: ["sign", "verify"],
        hash: "SHA-384",
      },
    ];

    for (const t of parameters) {
      const algorithm = { name: t.name, namedCurve: t.namedCurve };

      const { privateKey, publicKey } = await crypto.subtle.generateKey(
        algorithm,
        true,
        t.usages as webcrypto.KeyUsage[]
      );

      const keyAlgorithm = privateKey.algorithm as any;

      expect(keyAlgorithm.name).toEqual(algorithm.name);
      expect(keyAlgorithm.namedCurve).toEqual(algorithm.namedCurve);
      expect(privateKey.extractable).toEqual(false);

      expect(keyAlgorithm.name).toEqual(algorithm.name);
      expect(keyAlgorithm.namedCurve).toEqual(algorithm.namedCurve);
      expect(publicKey.extractable).toEqual(true);

      if (t.usages.includes("sign")) {
        const signature = await crypto.subtle.sign(
          {
            name: t.name,
            hash: t.hash,
          },
          privateKey,
          ENCODED_DATA
        );
        const isValid = await crypto.subtle.verify(
          {
            name: t.name,
            hash: t.hash,
          },
          publicKey,
          signature,
          ENCODED_DATA
        );

        expect(isValid).toBeTruthy();
      }
    }
  });

  it("should be processing Ed25519 algorithm", async () => {
    const parameters = [
      {
        name: "Ed25519",
        usages: ["sign", "verify"],
      },
    ];

    for (const t of parameters) {
      const algorithm = { name: t.name };

      const { privateKey, publicKey } = (await crypto.subtle.generateKey(
        algorithm,
        true,
        t.usages as webcrypto.KeyUsage[]
      )) as webcrypto.CryptoKeyPair;

      expect(privateKey.algorithm.name).toEqual(algorithm.name);
      expect(privateKey.extractable).toEqual(false);

      expect(publicKey.algorithm.name).toEqual(algorithm.name);
      expect(publicKey.extractable).toEqual(true);

      if (t.usages.includes("sign")) {
        const signature = await crypto.subtle.sign(
          {
            name: t.name,
          },
          privateKey,
          ENCODED_DATA
        );
        const isValid = await crypto.subtle.verify(
          {
            name: t.name,
          },
          publicKey,
          signature,
          ENCODED_DATA
        );

        expect(isValid).toBeTruthy();
      }
    }
  });

  it.skip("should be processing RSA-PSS/RSA-OAEP/RSASSA-PKCS1-v1_5 algorithm", async () => {
    const parameters = [
      {
        name: "RSA-PSS",
        modulusLength: 2048,
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-256",
        usages: ["sign", "verify"],
      },
      {
        name: "RSA-PSS",
        modulusLength: 2048,
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-384",
        usages: ["sign", "verify"],
      },
      {
        name: "RSA-PSS",
        modulusLength: 2048,
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-512",
        usages: ["sign", "verify"],
      },
      {
        name: "RSA-OAEP",
        modulusLength: 2048,
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-256",
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "RSA-OAEP",
        modulusLength: 2048,
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-384",
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "RSA-OAEP",
        modulusLength: 2048,
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-512",
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "RSASSA-PKCS1-v1_5",
        modulusLength: 2048,
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-256",
      },
      {
        name: "RSASSA-PKCS1-v1_5",
        modulusLength: 2048,
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-384",
      },
      {
        name: "RSASSA-PKCS1-v1_5",
        modulusLength: 2048,
        publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
        hash: "SHA-512",
      },
    ];

    for (const t of parameters) {
      const algorithm = {
        name: t.name,
        modulusLength: t.modulusLength,
        publicExponent: t.publicExponent,
        hash: t.hash,
      };

      const { privateKey, publicKey } = await crypto.subtle.generateKey(
        algorithm,
        true,
        t.usages as webcrypto.KeyUsage[]
      );

      const privateKeyAlgorithm = privateKey.algorithm as any;
      const publicKeyAlgorithm = publicKey.algorithm as any;

      expect(privateKey.algorithm.name).toEqual(t.name);
      expect(privateKeyAlgorithm.hash).toEqual({ name: algorithm.hash });
      expect(privateKey.extractable).toEqual(false);

      expect(publicKey.algorithm.name).toEqual(algorithm.name);
      expect(publicKeyAlgorithm.hash).toEqual({ name: algorithm.hash });
      expect(publicKey.extractable).toEqual(true);

      if (t.usages?.includes("sign")) {
        const signature = await crypto.subtle.sign(
          {
            name: t.name,
          },
          privateKey,
          ENCODED_DATA
        );
        const isValid = await crypto.subtle.verify(
          {
            name: t.name,
          },
          publicKey,
          signature,
          ENCODED_DATA
        );

        expect(isValid).toBeTruthy();
      }
    }
  }, 60000);
});

describe("SubtleCrypto generateKey/encrypt/decrypt", () => {
  it("should be processing AES-CBC algorithm", async () => {
    const parameters = [
      {
        name: "AES-CBC",
        length: 128,
        iv: crypto.getRandomValues(new Uint8Array(16)),
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CBC",
        length: 192,
        iv: crypto.getRandomValues(new Uint8Array(16)),
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CBC",
        length: 256,
        iv: crypto.getRandomValues(new Uint8Array(16)),
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
    ];

    for (const t of parameters) {
      const algorithm = { name: t.name, length: t.length };

      const key = await crypto.subtle.generateKey(
        algorithm,
        false,
        t.usages as webcrypto.KeyUsage[]
      );

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect((key.algorithm as any).length).toEqual(algorithm.length);
      expect(key.extractable).toEqual(false);

      if (t.usages.includes("encrypt")) {
        const encryptedData = await crypto.subtle.encrypt(
          {
            name: t.name,
            iv: t.iv,
          },
          key,
          ENCODED_DATA
        );
        const decryptedData = await crypto.subtle.decrypt(
          {
            name: t.name,
            iv: t.iv,
          },
          key,
          encryptedData
        );

        const result = DECODER.decode(decryptedData);
        expect(result).toEqual(TEST_MESSAGE);
      }
    }
  });
});

describe("SubtleCrypto generateKey/encrypt/decrypt", () => {
  it("should be processing AES-CTR algorithm", async () => {
    const parameters = [
      {
        name: "AES-CTR",
        length: 128,
        counter: crypto.getRandomValues(new Uint8Array(16)),
        counterLength: 32,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CTR",
        length: 128,
        counter: crypto.getRandomValues(new Uint8Array(16)),
        counterLength: 64,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CTR",
        length: 128,
        counter: crypto.getRandomValues(new Uint8Array(16)),
        counterLength: 128,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CTR",
        length: 192,
        counter: crypto.getRandomValues(new Uint8Array(16)),
        counterLength: 32,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CTR",
        length: 192,
        counter: crypto.getRandomValues(new Uint8Array(16)),
        counterLength: 64,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CTR",
        length: 192,
        counter: crypto.getRandomValues(new Uint8Array(16)),
        counterLength: 128,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CTR",
        length: 256,
        counter: crypto.getRandomValues(new Uint8Array(16)),
        counterLength: 32,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CTR",
        length: 256,
        counter: crypto.getRandomValues(new Uint8Array(16)),
        counterLength: 64,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-CTR",
        length: 256,
        counter: crypto.getRandomValues(new Uint8Array(16)),
        counterLength: 128,
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
    ];

    for (const t of parameters) {
      const algorithm = { name: t.name, length: t.length };

      const key = await crypto.subtle.generateKey(
        algorithm,
        false,
        t.usages as webcrypto.KeyUsage[]
      );

      const keyAlgorithm = key.algorithm as any;

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect(keyAlgorithm.length).toEqual(algorithm.length);
      expect(key.extractable).toEqual(false);

      if (t.usages.includes("encrypt")) {
        const encryptedData = await crypto.subtle.encrypt(
          {
            name: t.name,
            counter: t.counter,
            length: t.counterLength,
          },
          key,
          ENCODED_DATA
        );
        const decryptedData = await crypto.subtle.decrypt(
          {
            name: t.name,
            counter: t.counter,
            length: t.counterLength,
          },
          key,
          encryptedData
        );

        const result = DECODER.decode(decryptedData);
        expect(result).toEqual(TEST_MESSAGE);
      }
    }
  });
});

describe("SubtleCrypto generateKey/encrypt/decrypt", () => {
  it("should be processing AES-GCM algorithm", async () => {
    const parameters = [
      {
        name: "AES-GCM",
        length: 128,
        iv: crypto.getRandomValues(new Uint8Array(12)),
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-GCM",
        length: 192,
        iv: crypto.getRandomValues(new Uint8Array(12)),
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "AES-GCM",
        length: 256,
        iv: crypto.getRandomValues(new Uint8Array(12)),
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
    ];

    for (const t of parameters) {
      const algorithm = { name: t.name, length: t.length };

      const key = await crypto.subtle.generateKey(
        algorithm,
        false,
        t.usages as webcrypto.KeyUsage[]
      );

      const keyAlgorithm = key.algorithm as any;

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect(keyAlgorithm.length).toEqual(algorithm.length);
      expect(key.extractable).toEqual(false);

      if (t.usages.includes("encrypt")) {
        const encryptedData = await crypto.subtle.encrypt(
          {
            name: t.name,
            iv: t.iv,
          },
          key,
          ENCODED_DATA
        );
        const decryptedData = await crypto.subtle.decrypt(
          {
            name: t.name,
            iv: t.iv,
          },
          key,
          encryptedData
        );

        const result = DECODER.decode(decryptedData);
        expect(result).toEqual(TEST_MESSAGE);
      }
    }
  });
});

// Caveat: The current RSA implementation is too slow to complete the test within the time limit.
describe("SubtleCrypto generateKey/encrypt/decrypt", () => {
  it.skip("should be processing RSA-OAEP algorithm", async () => {
    const parameters = [
      {
        name: "RSA-OAEP",
        modulusLength: 2048,
        publicExponent: new Uint8Array([1, 0, 1]),
        hash: "SHA-256",
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "RSA-OAEP",
        modulusLength: 2048,
        publicExponent: new Uint8Array([1, 0, 1]),
        hash: "SHA-384",
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "RSA-OAEP",
        modulusLength: 2048,
        publicExponent: new Uint8Array([1, 0, 1]),
        hash: "SHA-512",
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
    ];

    for (const t of parameters) {
      const algorithm = {
        name: t.name,
        modulusLength: t.modulusLength,
        publicExponent: t.publicExponent,
        hash: t.hash,
      };

      const { privateKey, publicKey } = (await crypto.subtle.generateKey(
        algorithm,
        true,
        t.usages as webcrypto.KeyUsage[]
      )) as unknown as webcrypto.CryptoKeyPair;

      const privateKeyAlgorithm = privateKey.algorithm as any;
      const publicKeyAlgorithm = publicKey.algorithm as any;

      expect(privateKey.algorithm.name).toEqual(t.name);
      expect(privateKeyAlgorithm.hash).toEqual({ name: algorithm.hash });
      expect(privateKey.extractable).toEqual(false);

      expect(publicKey.algorithm.name).toEqual(algorithm.name);
      expect(publicKeyAlgorithm.hash).toEqual({ name: algorithm.hash });
      expect(publicKey.extractable).toEqual(true);

      if (t.usages.includes("encrypt")) {
        const encryptedData = await crypto.subtle.encrypt(
          {
            name: t.name,
          },
          publicKey,
          ENCODED_DATA
        );
        const decryptedData = await crypto.subtle.decrypt(
          {
            name: t.name,
          },
          privateKey,
          encryptedData
        );

        const result = DECODER.decode(decryptedData);
        expect(result).toEqual(TEST_MESSAGE);
      }
    }
  }, 60000);
});

describe("SubtleCrypto deriveBits/deriveKey", () => {
  it("should be processing ECDH algorithm", async () => {
    const generatedParams = [
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      {
        name: "ECDH",
        namedCurve: "P-384",
      },
    ];
    const derivedParams = [
      {
        name: "AES-CBC",
        length: 128,
      },
      {
        name: "AES-CBC",
        length: 192,
      },
      {
        name: "AES-CBC",
        length: 256,
      },
      {
        name: "AES-CTR",
        length: 128,
      },
      {
        name: "AES-CTR",
        length: 192,
      },
      {
        name: "AES-CTR",
        length: 256,
      },
      {
        name: "AES-GCM",
        length: 128,
      },
      {
        name: "AES-GCM",
        length: 192,
      },
      {
        name: "AES-GCM",
        length: 256,
      },
      {
        name: "AES-KW",
        length: 128,
      },
      {
        name: "AES-KW",
        length: 192,
      },
      {
        name: "AES-KW",
        length: 256,
      },
      {
        name: "HMAC",
        hash: "SHA-1",
      },
      {
        name: "HMAC",
        hash: "SHA-256",
      },
      {
        name: "HMAC",
        hash: "SHA-384",
      },
      {
        name: "HMAC",
        hash: "SHA-512",
      },
    ];

    // 1. Generate Alice's key pair
    const aliceKeyPair = await crypto.subtle.generateKey(
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      true, // whether the key is extractable (i.e. can be used in exportKey)
      ["deriveKey", "deriveBits"] // can be any combination of "deriveKey" and "deriveBits"
    );

    // 2. Generate Bob's key pair
    const bobKeyPair = await crypto.subtle.generateKey(
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      true,
      ["deriveKey", "deriveBits"]
    );

    // 3. Export Bob's public key to share with Alice
    const bobPublicKey = await crypto.subtle.exportKey(
      "raw",
      bobKeyPair.publicKey
    );

    // 3.5. Alice imports Bob's public key
    const bobImportKey = await crypto.subtle.importKey(
      "raw",
      bobPublicKey,
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      true,
      []
    );

    for (const generated of generatedParams) {
      for (const derived of derivedParams) {
        // 4. Alice derives a shared key using Bob's public key
        const aliceDerivedKey = await crypto.subtle.deriveKey(
          {
            name: "ECDH",
            public: bobImportKey,
          },
          aliceKeyPair.privateKey,
          derived,
          true, // The derived key is extractable
          ["encrypt", "decrypt"] // You can specify operations that the derived key will be used for
        );

        // 5. Export Alice's public key to share with Bob
        const alicePublicKey = await crypto.subtle.exportKey(
          "raw",
          aliceKeyPair.publicKey
        );

        // 6. Bob derives a shared key using Alice's public key
        const aliceImportKey = await crypto.subtle.importKey(
          "raw",
          alicePublicKey,
          {
            name: "ECDH",
            namedCurve: "P-256",
          },
          true,
          []
        );

        const bobDerivedKey = await crypto.subtle.deriveKey(
          {
            name: "ECDH",
            public: aliceImportKey,
          },
          bobKeyPair.privateKey,
          derived,
          true, // The derived key is extractable
          ["encrypt", "decrypt"] // You can specify operations that the derived key will be used for
        );

        // To verify if both Alice and Bob have the same derived key, you can compare them
        const aliceKeyBuffer = new Uint8Array(
          await crypto.subtle.exportKey("raw", aliceDerivedKey)
        );
        const bobKeyBuffer = new Uint8Array(
          await crypto.subtle.exportKey("raw", bobDerivedKey)
        );

        // Compare the raw key buffers to check if the derived keys are equal
        expect(aliceKeyBuffer).toEqual(bobKeyBuffer);
      }
    }
  });

  it("should be processing HKDF algorithm", async () => {
    const hkdfSalt = new Uint8Array(16); // Salt value (can be random, but here it's set to all zeros)
    const hkdfInfo = new TextEncoder().encode("HKDF info"); // Info parameter, can be any label string

    const generatedParams = [
      {
        name: "HKDF",
        salt: hkdfSalt,
        info: hkdfInfo,
        hash: "SHA-1",
      },
      {
        name: "HKDF",
        salt: hkdfSalt,
        info: hkdfInfo,
        hash: "SHA-256",
      },
      {
        name: "HKDF",
        salt: hkdfSalt,
        info: hkdfInfo,
        hash: "SHA-384",
      },
      {
        name: "HKDF",
        salt: hkdfSalt,
        info: hkdfInfo,
        hash: "SHA-512",
      },
    ];
    const derivedParams = [
      {
        name: "AES-CBC",
        length: 128,
      },
      {
        name: "AES-CBC",
        length: 192,
      },
      {
        name: "AES-CBC",
        length: 256,
      },
      {
        name: "AES-CTR",
        length: 128,
      },
      {
        name: "AES-CTR",
        length: 192,
      },
      {
        name: "AES-CTR",
        length: 256,
      },
      {
        name: "AES-GCM",
        length: 128,
      },
      {
        name: "AES-GCM",
        length: 192,
      },
      {
        name: "AES-GCM",
        length: 256,
      },
      {
        name: "AES-KW",
        length: 128,
      },
      {
        name: "AES-KW",
        length: 192,
      },
      {
        name: "AES-KW",
        length: 256,
      },
      {
        name: "HMAC",
        hash: "SHA-1",
      },
      {
        name: "HMAC",
        hash: "SHA-256",
      },
      {
        name: "HMAC",
        hash: "SHA-384",
      },
      {
        name: "HMAC",
        hash: "SHA-512",
      },
    ];

    // 1. Generate Alice's key pair
    const aliceKeyPair = await crypto.subtle.generateKey(
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      true, // whether the key is extractable (i.e. can be used in exportKey)
      ["deriveKey", "deriveBits"] // can be any combination of "deriveKey" and "deriveBits"
    );

    // 2. Generate Bob's key pair
    const bobKeyPair = await crypto.subtle.generateKey(
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      true,
      ["deriveKey", "deriveBits"]
    );

    // 3. Export Bob's public key to share with Alice
    const bobPublicKey = await crypto.subtle.exportKey(
      "raw",
      bobKeyPair.publicKey
    );

    // 3.5. Alice imports Bob's public key
    const bobImportKey = await crypto.subtle.importKey(
      "raw",
      bobPublicKey,
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      true,
      []
    );

    for (const generated of generatedParams) {
      for (const derived of derivedParams) {
        // 4. Alice derives a shared secret using Bob's public key
        const aliceSharedSecret = await crypto.subtle.deriveBits(
          {
            name: "ECDH",
            public: bobImportKey,
          },
          aliceKeyPair.privateKey,
          256 // number of bits to derive
        );

        // 5. Convert Alice's derived secret to a key using HKDF
        const aliceDerivedKey = await crypto.subtle.importKey(
          "raw",
          aliceSharedSecret,
          "HKDF",
          false,
          ["deriveKey"]
        );

        const aliceDerivedlKey = await crypto.subtle.deriveKey(
          generated,
          aliceDerivedKey,
          derived,
          true,
          ["encrypt", "decrypt"]
        );

        // 6. Export Alice's public key to share with Bob
        const alicePublicKey = await crypto.subtle.exportKey(
          "raw",
          aliceKeyPair.publicKey
        );

        // 7. Bob derives a shared secret using Alice's public key
        const aliceImportKey = await crypto.subtle.importKey(
          "raw",
          alicePublicKey,
          {
            name: "ECDH",
            namedCurve: "P-256",
          },
          true,
          []
        );

        const bobSharedSecret = await crypto.subtle.deriveBits(
          {
            name: "ECDH",
            public: aliceImportKey,
          },
          bobKeyPair.privateKey,
          256
        );

        // 8. Convert Bob's derived secret to a key using HKDF
        const bobDerivedKey = await crypto.subtle.importKey(
          "raw",
          bobSharedSecret,
          "HKDF",
          false,
          ["deriveKey"]
        );

        const bobDerivedlKey = await crypto.subtle.deriveKey(
          generated,
          bobDerivedKey,
          derived,
          true,
          ["encrypt", "decrypt"]
        );
        console.log("8. Bob derives the final key using HKDF : complete");

        // 9. Verify if both derived keys are the same
        const aliceKeyBuffer = new Uint8Array(
          await crypto.subtle.exportKey("raw", aliceDerivedlKey)
        );
        const bobKeyBuffer = new Uint8Array(
          await crypto.subtle.exportKey("raw", bobDerivedlKey)
        );

        // Compare the raw key buffers to check if the derived keys are equal
        expect(aliceKeyBuffer).toEqual(bobKeyBuffer);
      }
    }
  });

  it("should be processing PBKDF2 algorithm", async () => {
    const pbkdf2Salt = new Uint8Array(16); // Salt value (can be random, but here it's set to all zeros)
    const pbkdf2Iterations = 50000; // Number of iterations for PBKDF2

    // We skip some tests because they run slowly in CI.
    const generatedParams = [
      {
        //   name: "PBKDF2",
        //   salt: pbkdf2Salt,
        //   iterations: pbkdf2Iterations,
        //   hash: "SHA-1"
        // }, {
        name: "PBKDF2",
        salt: pbkdf2Salt,
        iterations: pbkdf2Iterations,
        hash: "SHA-256",
        // }, {
        //   name: "PBKDF2",
        //   salt: pbkdf2Salt,
        //   iterations: pbkdf2Iterations,
        //   hash: "SHA-384"
        // }, {
        // name: "PBKDF2",
        // salt: pbkdf2Salt,
        // iterations: pbkdf2Iterations,
        // hash: "SHA-512"
      },
    ];
    const derivedParams = [
      {
        name: "AES-CBC",
        length: 128,
      },
      {
        //   name: "AES-CBC",
        //   length: 192,
        // }, {
        // name: "AES-CBC",
        // length: 256,
        // }, {
        name: "AES-CTR",
        length: 128,
      },
      {
        //   name: "AES-CTR",
        //   length: 192,
        // }, {
        // name: "AES-CTR",
        // length: 256,
        // }, {
        name: "AES-GCM",
        length: 128,
      },
      {
        //   name: "AES-GCM",
        //   length: 192,
        // }, {
        //   name: "AES-GCM",
        //   length: 256,
        // }, {
        name: "AES-KW",
        length: 128,
      },
      {
        //   name: "AES-KW",
        //   length: 192,
        // }, {
        //   name: "AES-KW",
        //   length: 256,
        // }, {
        name: "HMAC",
        hash: "SHA-1",
        // }, {
        //   name: "HMAC",
        //   hash: "SHA-256",
        // }, {
        //   name: "HMAC",
        //   hash: "SHA-384",
        // }, {
        // name: "HMAC",
        // hash: "SHA-512",
      },
    ];

    // 1. Generate Alice's key pair
    const aliceKeyPair = await crypto.subtle.generateKey(
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      true, // whether the key is extractable (i.e. can be used in exportKey)
      ["deriveKey", "deriveBits"] // can be any combination of "deriveKey" and "deriveBits"
    );

    // 2. Generate Bob's key pair
    const bobKeyPair = await crypto.subtle.generateKey(
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      true,
      ["deriveKey", "deriveBits"]
    );

    // 3. Export Bob's public key to share with Alice
    const bobPublicKey = await crypto.subtle.exportKey(
      "raw",
      bobKeyPair.publicKey
    );

    // 3.5. Alice imports Bob's public key
    const bobImportKey = await crypto.subtle.importKey(
      "raw",
      bobPublicKey,
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      true,
      []
    );

    for (const generated of generatedParams) {
      for (const derived of derivedParams) {
        // 4. Alice derives a shared secret using Bob's public key
        const aliceSharedSecret = await crypto.subtle.deriveBits(
          {
            name: "ECDH",
            public: bobImportKey,
          },
          aliceKeyPair.privateKey,
          256 // number of bits to derive
        );

        // Use PBKDF2 to generate a derived key from Alice's shared secret
        const aliceFinalKey = await crypto.subtle.importKey(
          "raw",
          aliceSharedSecret,
          "PBKDF2",
          false,
          ["deriveKey"]
        );

        const aliceDerivedKey = await crypto.subtle.deriveKey(
          generated,
          aliceFinalKey,
          derived,
          true,
          ["encrypt", "decrypt"]
        );

        // 6. Export Alice's public key to share with Bob
        const alicePublicKey = await crypto.subtle.exportKey(
          "raw",
          aliceKeyPair.publicKey
        );

        // 7. Bob derives a shared secret using Alice's public key
        const aliceImportKey = await crypto.subtle.importKey(
          "raw",
          alicePublicKey,
          {
            name: "ECDH",
            namedCurve: "P-256",
          },
          true,
          []
        );

        const bobSharedSecret = await crypto.subtle.deriveBits(
          {
            name: "ECDH",
            public: aliceImportKey,
          },
          bobKeyPair.privateKey,
          256
        );

        // 8. Convert Bob's derived secret to a key using PBKDF2
        const bobFinalKey = await crypto.subtle.importKey(
          "raw",
          bobSharedSecret,
          "PBKDF2",
          false,
          ["deriveKey"]
        );

        const bobDerivedKey = await crypto.subtle.deriveKey(
          generated,
          bobFinalKey,
          derived,
          true,
          ["encrypt", "decrypt"]
        );

        // 9. Verify if both derived keys are the same
        const aliceKeyBuffer = new Uint8Array(
          await crypto.subtle.exportKey("raw", aliceDerivedKey)
        );
        const bobKeyBuffer = new Uint8Array(
          await crypto.subtle.exportKey("raw", bobDerivedKey)
        );

        // Compare the raw key buffers to check if the derived keys are equal
        expect(aliceKeyBuffer).toEqual(bobKeyBuffer);
      }
    }
  });
});
