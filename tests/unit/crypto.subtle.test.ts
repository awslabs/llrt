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
  // Common test parameters
  const keyLengths = [128, 192, 256];
  const hashAlgorithms = ["SHA-1", "SHA-256", "SHA-384", "SHA-512"];
  const curves = ["P-256", "P-384"];
  const rsaParams = {
    modulusLength: 2048,
    publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
  };

  it("should be processing AES-CBC/AES-CTR/AES-GCM/AES-KW algorithm", async () => {
    const aesAlgorithms = ["AES-CBC", "AES-CTR", "AES-GCM"];
    const aesUsages = ["encrypt", "decrypt", "wrapKey", "unwrapKey"];
    const kwUsages = ["wrapKey", "unwrapKey"];

    const parameters = aesAlgorithms.flatMap((name) =>
      keyLengths.map((length) => ({
        name,
        length,
        usages: name === "AES-KW" ? kwUsages : aesUsages,
      }))
    );
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
    const parameters = hashAlgorithms.map((hash) => ({
      name: "HMAC",
      hash,
      usages: ["sign", "verify"],
    }));

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
    const parameters: {
      name: string;
      namedCurve: string;
      usages: string[];
      hash?: string;
    }[] = [
      ...curves.map((curve, i) => ({
        name: "ECDSA",
        namedCurve: curve,
        usages: ["sign", "verify"],
        hash: i === 0 ? "SHA-256" : "SHA-384",
      })),
      ...curves.map((curve) => ({
        name: "ECDH",
        namedCurve: curve,
        usages: ["deriveKey", "deriveBits"],
      })),
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
      expect(privateKey.extractable).toEqual(true);
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
      expect(privateKey.extractable).toEqual(true);
      expect(publicKey.extractable).toEqual(true);

      if (t.usages.includes("sign")) {
        const signature = await crypto.subtle.sign(
          { name: t.name },
          privateKey,
          ENCODED_DATA
        );
        const isValid = await crypto.subtle.verify(
          { name: t.name },
          publicKey,
          signature,
          ENCODED_DATA
        );

        expect(isValid).toBeTruthy();
      }
    }
  });

  it.skip("should be processing RSA-PSS/RSA-OAEP/RSASSA-PKCS1-v1_5 algorithm", async () => {
    const rsaAlgorithms = [
      {
        name: "RSA-PSS",
        usages: ["sign", "verify"],
      },
      {
        name: "RSA-OAEP",
        usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
      },
      {
        name: "RSASSA-PKCS1-v1_5",
        usages: ["sign", "verify"],
      },
    ];

    const parameters = rsaAlgorithms.flatMap((algo) =>
      hashAlgorithms
        .filter((h) => h !== "SHA-1")
        .map((hash) => ({
          ...algo,
          ...rsaParams,
          hash,
        }))
    );

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
      expect(privateKey.extractable).toEqual(true);

      expect(publicKey.algorithm.name).toEqual(algorithm.name);
      expect(publicKeyAlgorithm.hash).toEqual({ name: algorithm.hash });
      expect(publicKey.extractable).toEqual(true);

      if (t.usages?.includes("sign")) {
        const signature = await crypto.subtle.sign(
          { name: t.name },
          privateKey,
          ENCODED_DATA
        );
        const isValid = await crypto.subtle.verify(
          { name: t.name },
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
  // Common key lengths and usages for AES algorithms
  const keyLengths = [128, 192, 256];
  const commonUsages = ["encrypt", "decrypt", "wrapKey", "unwrapKey"];

  it("should be processing AES-CBC algorithm", async () => {
    const parameters = keyLengths.map((length) => ({
      name: "AES-CBC",
      length,
      iv: crypto.getRandomValues(new Uint8Array(16)),
      usages: commonUsages,
    }));

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

  it("should be processing AES-CTR algorithm", async () => {
    const counterLengths = [32, 64, 128];
    const parameters = keyLengths.flatMap((length) =>
      counterLengths.map((counterLength) => ({
        name: "AES-CTR",
        length,
        counter: crypto.getRandomValues(new Uint8Array(16)),
        counterLength,
        usages: commonUsages,
      }))
    );

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

  it("should be processing AES-GCM algorithm", async () => {
    const parameters = keyLengths.map((length) => ({
      name: "AES-GCM",
      length,
      iv: crypto.getRandomValues(new Uint8Array(12)),
      usages: commonUsages,
    }));

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

  // Caveat: The current RSA implementation is too slow to complete the test within the time limit.
  it.skip("should be processing RSA-OAEP algorithm", async () => {
    const hashAlgorithms = ["SHA-256", "SHA-384", "SHA-512"];
    const parameters = hashAlgorithms.map((hash) => ({
      name: "RSA-OAEP",
      modulusLength: 2048,
      publicExponent: new Uint8Array([1, 0, 1]),
      hash,
      usages: commonUsages,
    }));

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
      expect(privateKey.extractable).toEqual(true);

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
    const keyLengths = [128, 192, 256];
    const algorithms = ["AES-CBC", "AES-CTR", "AES-GCM"];

    const namedCurves = ["P-256", "P-384"];

    const derivedParams = algorithms.flatMap((name) =>
      keyLengths.map((length) => ({
        name,
        length,
      }))
    );

    for (const namedCurve of namedCurves) {
      // 1. Generate Alice's key pair
      const aliceKeyPair = await crypto.subtle.generateKey(
        {
          name: "ECDH",
          namedCurve,
        },
        true, // whether the key is extractable (i.e. can be used in exportKey)
        ["deriveKey", "deriveBits"] // can be any combination of "deriveKey" and "deriveBits"
      );

      // 2. Generate Bob's key pair
      const bobKeyPair = await crypto.subtle.generateKey(
        {
          name: "ECDH",
          namedCurve,
        },
        true,
        ["deriveKey", "deriveBits"]
      );

      for (const derived of derivedParams) {
        // 4. Alice derives a shared key using Bob's public key
        const aliceDerivedKey = await crypto.subtle.deriveKey(
          {
            name: "ECDH",
            public: bobKeyPair.publicKey,
          },
          aliceKeyPair.privateKey,
          derived,
          true, // The derived key is extractable
          ["encrypt", "decrypt"] // You can specify operations that the derived key will be used for
        );

        const bobDerivedKey = await crypto.subtle.deriveKey(
          {
            name: "ECDH",
            // public: aliceImportKey,
            public: aliceKeyPair.publicKey,
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

    const keyLengths = [128, 192, 256];
    const algorithms = ["AES-CBC", "AES-CTR", "AES-GCM"];
    const hashAlgorithms = ["SHA-1", "SHA-256", "SHA-384", "SHA-512"];

    const generatedParams = hashAlgorithms.map((hash) => ({
      name: "HKDF",
      salt: hkdfSalt,
      info: hkdfInfo,
      hash,
    }));

    const derivedParams = algorithms.flatMap((name) =>
      keyLengths.map((length) => ({
        name,
        length,
      }))
    );

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

    for (const generated of generatedParams) {
      for (const derived of derivedParams) {
        // 4. Alice derives a shared secret using Bob's public key
        const aliceSharedSecret = await crypto.subtle.deriveBits(
          {
            name: "ECDH",
            public: bobKeyPair.publicKey,
          },
          aliceKeyPair.privateKey,
          256 // number of bits to derive
        );

        // 5. Convert Alice's derived secret to a key using HKDF
        const aliceImportedKey = await crypto.subtle.importKey(
          "raw",
          aliceSharedSecret,
          "HKDF",
          false,
          ["deriveKey"]
        );

        const aliceDerivedKey = await crypto.subtle.deriveKey(
          generated,
          aliceImportedKey,
          derived,
          true,
          ["encrypt", "decrypt"]
        );

        const bobSharedSecret = await crypto.subtle.deriveBits(
          {
            name: "ECDH",
            public: aliceKeyPair.publicKey,
          },
          bobKeyPair.privateKey,
          256
        );

        // 8. Convert Bob's derived secret to a key using HKDF
        const bobImportedKey = await crypto.subtle.importKey(
          "raw",
          bobSharedSecret,
          "HKDF",
          false,
          ["deriveKey"]
        );

        const bobDerivedKey = await crypto.subtle.deriveKey(
          generated,
          bobImportedKey,
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

  it("should be processing PBKDF2 algorithm", async () => {
    const pbkdf2Salt = new Uint8Array(16); // Salt value (can be random, but here it's set to all zeros)
    const pbkdf2Iterations = 50; // Number of iterations for PBKDF2

    const hashAlgorithms = ["SHA-1", "SHA-256", "SHA-384", "SHA-512"];
    const generatedParams = hashAlgorithms.map((hash) => ({
      name: "PBKDF2",
      salt: pbkdf2Salt,
      iterations: pbkdf2Iterations,
      hash,
    }));

    const algorithms = ["AES-CBC", "AES-CTR", "AES-GCM"];
    const keyLengths = [128, 192, 256];
    const derivedParams = algorithms.flatMap((name) =>
      keyLengths.map((length) => ({
        name,
        length,
      }))
    );
    //.filter(({ name, length }) => !(name === "AES-GCM" && length > 128)); // Only include AES-GCM-128
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

    for (const generated of generatedParams) {
      for (const derived of derivedParams) {
        // 4. Alice derives a shared secret using Bob's public key

        const aliceSharedSecret = await crypto.subtle.deriveBits(
          {
            name: "ECDH",
            // public: bobImportKey,
            public: bobKeyPair.publicKey,
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

        const bobSharedSecret = await crypto.subtle.deriveBits(
          {
            name: "ECDH",
            public: aliceKeyPair.publicKey,
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

describe("SubtileCrypto import/export", () => {
  it("should export and import keys", async () => {
    // Test different key algorithms and formats
    // Define reusable constants
    const SYMMETRIC_FORMATS: webcrypto.KeyFormat[] = ["raw", "jwk"];
    const ASYMMETRIC_FORMATS: webcrypto.KeyFormat[] = ["pkcs8", "spki", "jwk"];
    const HASH_ALGORITHMS = ["SHA-1", "SHA-256", "SHA-384", "SHA-512"];
    const EC_CURVES = ["P-256", "P-384", "P-521"];
    const AES_LENGTHS = [128, 192, 256];

    const algorithms = [
      // AES algorithms
      ...["AES-CBC", "AES-GCM", "AES-CTR"].flatMap((name) =>
        AES_LENGTHS.map((length) => ({
          generateParams: {
            name,
            length,
          },
          usages: ["encrypt", "decrypt"],
          formats: SYMMETRIC_FORMATS,
        }))
      ),

      // RSA algorithms
      // ...["RSASSA-PKCS1-v1_5", "RSA-PSS", "RSA-OAEP"].flatMap((name) =>
      //   HASH_ALGORITHMS.map((hash) => ({
      //     generateParams: {
      //       name,
      //       modulusLength: 2048,
      //       publicExponent: new Uint8Array([1, 0, 1]),
      //       hash,
      //     },
      //     usages:
      //       name === "RSA-OAEP" ? ["encrypt", "decrypt"] : ["sign", "verify"],
      //     formats: ASYMMETRIC_FORMATS,
      //   }))
      // ),

      // EC algorithms
      ...["ECDSA", "ECDH"].flatMap((name) =>
        EC_CURVES.map((namedCurve) => ({
          generateParams: {
            name,
            namedCurve,
          },
          usages:
            name === "ECDH" ? ["deriveKey", "deriveBits"] : ["sign", "verify"],
          formats: ASYMMETRIC_FORMATS,
        }))
      ),

      // HMAC
      ...HASH_ALGORITHMS.map((hash) => ({
        generateParams: {
          name: "HMAC",
          hash,
        },
        usages: ["sign", "verify"],
        formats: SYMMETRIC_FORMATS,
      })),
    ];
    for (const algorithm of algorithms) {
      // Generate key
      const key = (await crypto.subtle.generateKey(
        algorithm.generateParams,
        true,
        algorithm.usages as webcrypto.KeyUsage[]
      )) as webcrypto.CryptoKeyPair & webcrypto.CryptoKey;

      // For asymmetric keys, test both public and private keys
      const keys = algorithm.formats.includes("spki")
        ? [key.publicKey, key.privateKey]
        : [key];

      for (const k of keys) {
        for (const format of algorithm.formats) {
          if (k.type == "public" && format == "pkcs8") {
            return;
          }
          if (k.type == "private" && format == "spki") {
            return;
          }
          // Export key
          const exported = (await crypto.subtle.exportKey(
            format as any,
            k
          )) as any;

          // Import key back
          const imported = await crypto.subtle.importKey(
            format as any,
            exported,
            algorithm.generateParams,
            true,
            algorithm.usages as webcrypto.KeyUsage[]
          );

          // Export both keys again to compare
          const exportedOriginal = (await crypto.subtle.exportKey(
            format as any,
            k
          )) as any;
          const exportedImported = (await crypto.subtle.exportKey(
            format as any,
            imported
          )) as any;

          // Compare the exported keys
          if (format === "jwk") {
            expect(exportedImported).toEqual(exportedOriginal);
          } else {
            const originalBuffer = new Uint8Array(exportedOriginal);
            const importedBuffer = new Uint8Array(exportedImported);
            expect(importedBuffer).toEqual(originalBuffer);
          }
        }
      }
    }
  }, 30000);
});

describe.only("SubtileCrypto wrap/unwrap", () => {
  it("should wrap and unwrap keys for all supported algorithms", async () => {
    // Test parameters
    const HASH_ALGORITHMS = ["SHA-1", "SHA-256", "SHA-384", "SHA-512"];
    const AES_LENGTHS = [128, 192, 256];
    const EC_CURVES = ["P-256", "P-384", "P-521"];

    // Wrapping algorithms
    const wrappingAlgorithms = [
      // AES-KW
      ...AES_LENGTHS.map((length) => ({
        name: "AES-KW",
        generateParams: {
          name: "AES-KW",
          length,
        },
        wrapParams: {
          name: "AES-KW",
        },
        usages: ["wrapKey", "unwrapKey"],
      })),

      // RSA-OAEP
      // ...HASH_ALGORITHMS.map((hash) => ({
      //   name: "RSA-OAEP",
      //   generateParams: {
      //     name: "RSA-OAEP",
      //     modulusLength: 2048,
      //     publicExponent: new Uint8Array([1, 0, 1]),
      //     hash,
      //   },
      //   wrapParams: {
      //     name: "RSA-OAEP",
      //   },
      //   usages: ["wrapKey", "unwrapKey"],
      // })),
    ];

    // Keys to be wrapped
    const keysToWrap = [
      // AES keys
      ...["AES-CBC", "AES-GCM", "AES-CTR"].flatMap((name) =>
        AES_LENGTHS.map((length) => ({
          generateParams: {
            name,
            length,
          },
          usages: ["encrypt", "decrypt"],
        }))
      ),

      // HMAC keys
      ...HASH_ALGORITHMS.map((hash) => ({
        generateParams: {
          name: "HMAC",
          hash,
        },
        usages: ["sign", "verify"],
      })),

      // EC keys
      ...["ECDSA", "ECDH"].flatMap((name) =>
        EC_CURVES.map((namedCurve) => ({
          generateParams: {
            name,
            namedCurve,
          },
          usages:
            name === "ECDH" ? ["deriveKey", "deriveBits"] : ["sign", "verify"],
        }))
      ),
    ];

    for (const wrappingAlg of wrappingAlgorithms) {
      // Generate wrapping key pair for RSA-OAEP or single key for AES-KW
      const wrappingKey = (await crypto.subtle.generateKey(
        wrappingAlg.generateParams,
        true,
        wrappingAlg.usages as webcrypto.KeyUsage[]
      )) as webcrypto.CryptoKeyPair & webcrypto.CryptoKey;

      for (const keyToWrap of keysToWrap) {
        // Generate key to be wrapped
        const originalKey = (await crypto.subtle.generateKey(
          keyToWrap.generateParams,
          true,
          keyToWrap.usages as webcrypto.KeyUsage[]
        )) as webcrypto.CryptoKeyPair & webcrypto.CryptoKey;

        // For asymmetric keys, test both public and private keys
        const keysToTest = originalKey.publicKey
          ? [originalKey.publicKey, originalKey.privateKey]
          : [originalKey];

        for (const keyToTest of keysToTest) {
          const wrappedKey = await crypto.subtle.wrapKey(
            "jwk",
            keyToTest,
            wrappingAlg.name === "RSA-OAEP"
              ? wrappingKey.publicKey
              : wrappingKey,
            wrappingAlg.wrapParams
          );

          // Unwrap the key
          const unwrappedKey = await crypto.subtle.unwrapKey(
            "jwk",
            wrappedKey,
            wrappingAlg.name === "RSA-OAEP"
              ? wrappingKey.privateKey
              : wrappingKey,
            wrappingAlg.wrapParams,
            keyToWrap.generateParams,
            true,
            keyToWrap.usages as webcrypto.KeyUsage[]
          );

          // Export both keys to compare
          const originalExported = await crypto.subtle.exportKey(
            "raw",
            keyToTest
          );
          const unwrappedExported = await crypto.subtle.exportKey(
            "raw",
            unwrappedKey
          );

          // Compare the exported keys
          const originalBuffer = new Uint8Array(originalExported);
          const unwrappedBuffer = new Uint8Array(unwrappedExported);
          expect(unwrappedBuffer).toEqual(originalBuffer);
        }
      }
    }
  }, 30000);
});
