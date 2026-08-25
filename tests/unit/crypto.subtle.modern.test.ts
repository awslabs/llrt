import { createHash } from "node:crypto";
import type { webcrypto } from "node:crypto";

const DATA = new TextEncoder().encode("modern WebCrypto");

describe("Modern WebCrypto key formats", () => {
  it("aliases raw public and secret key formats", async () => {
    const hmac = (await crypto.subtle.generateKey(
      { name: "HMAC", hash: "SHA-256" },
      true,
      ["sign"]
    )) as webcrypto.CryptoKey;
    const secret = await crypto.subtle.exportKey("raw-secret", hmac);
    const importedSecret = await crypto.subtle.importKey(
      "raw-secret",
      secret,
      { name: "HMAC", hash: "SHA-256" },
      true,
      ["sign"]
    );
    expect(await crypto.subtle.exportKey("raw", importedSecret)).toEqual(
      secret
    );

    const keyPair = await crypto.subtle.generateKey("Ed25519", true, [
      "verify",
    ]);
    const publicKey = await crypto.subtle.exportKey(
      "raw-public",
      keyPair.publicKey
    );
    const importedPublic = await crypto.subtle.importKey(
      "raw-public",
      publicKey,
      "Ed25519",
      true,
      ["verify"]
    );
    expect(await crypto.subtle.exportKey("raw", importedPublic)).toEqual(
      publicKey
    );
  });

  it("rejects recognized key formats unsupported by an algorithm", async () => {
    for (const format of ["raw-private", "raw-seed"] as const) {
      await expect(
        crypto.subtle.importKey(
          format,
          new Uint8Array([1]),
          { name: "HMAC", hash: "SHA-256" },
          false,
          ["sign"]
        )
      ).rejects.toHaveProperty("name", "NotSupportedError");
    }
  });

  it("rejects raw-public export of private keys as invalid access", async () => {
    const keyPair = await crypto.subtle.generateKey(
      { name: "ECDSA", namedCurve: "P-256" },
      true,
      ["sign", "verify"]
    );
    await expect(
      crypto.subtle.exportKey("raw-public", keyPair.privateKey)
    ).rejects.toHaveProperty("name", "InvalidAccessError");

    const wrappingKey = await crypto.subtle.generateKey(
      { name: "AES-KW", length: 128 },
      false,
      ["wrapKey"]
    );
    await expect(
      crypto.subtle.wrapKey(
        "raw-public",
        keyPair.privateKey,
        wrappingKey,
        "AES-KW"
      )
    ).rejects.toHaveProperty("name", "InvalidAccessError");
  });
});

describe("Modern WebCrypto SHA-3", () => {
  it("keeps SHA-3 out of Node streaming hash names", () => {
    expect(() => createHash("SHA3-256")).toThrow();
  });

  it("calculates fixed SHA-3 digests", async () => {
    const result = await crypto.subtle.digest("SHA3-256", new Uint8Array());
    expect(new Uint8Array(result)).toEqual(
      new Uint8Array([
        167, 255, 198, 248, 191, 30, 215, 102, 81, 193, 71, 86, 160, 97, 214,
        98, 245, 128, 255, 77, 228, 59, 73, 250, 130, 216, 10, 75, 128, 248, 67,
        74,
      ])
    );
  });

  it("keeps SHA-3 out of HMAC", async () => {
    await expect(
      crypto.subtle.generateKey(
        { name: "HMAC", hash: "SHA3-256" },
        false,
        ["sign"]
      )
    ).rejects.toHaveProperty("name", "NotSupportedError");
  });
});

describe("Modern WebCrypto XOF digests", () => {
  it("matches cSHAKE and TurboSHAKE vectors", async () => {
    const vectors = [
      [
        { name: "cSHAKE128", outputLength: 256 },
        "7f9c2ba4e88f827d616045507605853ed73b8093f6efbc88eb1a6eacfa66ef26",
      ],
      [
        { name: "TurboSHAKE128", outputLength: 256 },
        "1e415f1c5983aff2169217277d17bb538cd945a397ddec541f1ce41af2c1b74c",
      ],
    ] as const;

    for (const [algorithm, expected] of vectors) {
      const digest = new Uint8Array(
        await crypto.subtle.digest(algorithm, new Uint8Array())
      );
      expect(Buffer.from(digest).toString("hex")).toBe(expected);
    }
  });

  it("supports partial-bit cSHAKE output", async () => {
    const digest = new Uint8Array(
      await crypto.subtle.digest(
        { name: "cSHAKE256", outputLength: 9 },
        new Uint8Array()
      )
    );
    expect(digest.byteLength).toBe(2);
    expect(digest[1] & 0x7f).toBe(0);
  });

  it("validates XOF parameters", async () => {
    expect(
      await crypto.subtle.digest(
        { name: "cSHAKE128", outputLength: null as unknown as number },
        new Uint8Array()
      )
    ).toHaveProperty("byteLength", 0);

    await expect(
      crypto.subtle.digest(
        { name: "TurboSHAKE128", outputLength: null as unknown as number },
        new Uint8Array()
      )
    ).rejects.toMatchObject({ name: "OperationError" });

    await expect(
      crypto.subtle.digest(
        { name: "cSHAKE128", outputLength: 0xffffffff },
        new Uint8Array()
      )
    ).rejects.toMatchObject({ name: "OperationError" });

    for (const domainSeparation of [0, 0x80, null]) {
      await expect(
        crypto.subtle.digest(
          {
            name: "TurboSHAKE128",
            outputLength: 256,
            domainSeparation: domainSeparation as unknown as number,
          },
          new Uint8Array()
        )
      ).rejects.toMatchObject({ name: "OperationError" });
    }
  });

  it("rejects null cSHAKE BufferSource parameters", async () => {
    for (const parameter of ["functionName", "customization"] as const) {
      await expect(
        crypto.subtle.digest(
          {
            name: "cSHAKE128",
            outputLength: 256,
            [parameter]: null,
          },
          new Uint8Array()
        )
      ).rejects.toThrow(TypeError);
    }
  });
});

describe("Modern WebCrypto ChaCha20-Poly1305", () => {
  it("encrypts, decrypts, and authenticates additional data", async () => {
    const key = await crypto.subtle.generateKey("ChaCha20-Poly1305", true, [
      "encrypt",
      "decrypt",
    ]);
    const algorithm = {
      name: "ChaCha20-Poly1305",
      iv: new Uint8Array(12),
      additionalData: new Uint8Array([1, 2, 3]),
    };
    const ciphertext = await crypto.subtle.encrypt(algorithm, key, DATA);
    expect(
      new Uint8Array(await crypto.subtle.decrypt(algorithm, key, ciphertext))
    ).toEqual(DATA);

    const tampered = new Uint8Array(ciphertext);
    tampered[0] ^= 1;
    await expect(
      crypto.subtle.decrypt(algorithm, key, tampered)
    ).rejects.toHaveProperty("name", "OperationError");
  });

  it("round-trips raw-secret and JWK keys", async () => {
    const key = await crypto.subtle.generateKey("ChaCha20-Poly1305", true, [
      "encrypt",
      "decrypt",
    ]);
    const raw = await crypto.subtle.exportKey("raw-secret", key);
    expect(raw.byteLength).toBe(32);
    const jwk = await crypto.subtle.exportKey("jwk", key);
    expect(jwk).toMatchObject({ kty: "oct", alg: "C20P" });

    for (const [format, data] of [
      ["raw-secret", raw],
      ["jwk", jwk],
    ] as const) {
      const imported = await crypto.subtle.importKey(
        format,
        data,
        "ChaCha20-Poly1305",
        true,
        ["encrypt"]
      );
      expect(await crypto.subtle.exportKey("raw-secret", imported)).toEqual(
        raw
      );
    }
  });

  it("derives 256-bit keys", async () => {
    const baseKey = await crypto.subtle.importKey(
      "raw-secret",
      new Uint8Array([1]),
      "HKDF",
      false,
      ["deriveKey"]
    );
    const key = await crypto.subtle.deriveKey(
      {
        name: "HKDF",
        hash: "SHA-256",
        salt: new Uint8Array(),
        info: new Uint8Array(),
      },
      baseKey,
      "ChaCha20-Poly1305",
      true,
      ["encrypt"]
    );

    expect(key.algorithm).toMatchObject({ name: "ChaCha20-Poly1305" });
    expect((await crypto.subtle.exportKey("raw-secret", key)).byteLength).toBe(
      32
    );
  });

  it("validates key and AEAD parameters", async () => {
    await expect(
      crypto.subtle.importKey(
        "raw-secret",
        new Uint8Array(31),
        "ChaCha20-Poly1305",
        false,
        ["encrypt"]
      )
    ).rejects.toHaveProperty("name", "DataError");

    const key = await crypto.subtle.generateKey("ChaCha20-Poly1305", false, [
      "encrypt",
      "decrypt",
    ]);
    for (const algorithm of [
      { name: "ChaCha20-Poly1305", iv: new Uint8Array(11) },
      {
        name: "ChaCha20-Poly1305",
        iv: new Uint8Array(12),
        tagLength: 96,
      },
      {
        name: "ChaCha20-Poly1305",
        iv: new Uint8Array(12),
        tagLength: null as unknown as number,
      },
    ]) {
      await expect(
        crypto.subtle.encrypt(algorithm, key, DATA)
      ).rejects.toHaveProperty("name", "OperationError");
    }
  });

  it("wraps and unwraps keys", async () => {
    const wrappingKey = await crypto.subtle.generateKey(
      "ChaCha20-Poly1305",
      false,
      ["wrapKey", "unwrapKey"]
    );
    const target = await crypto.subtle.generateKey(
      { name: "AES-GCM", length: 128 },
      true,
      ["encrypt"]
    );
    const algorithm = {
      name: "ChaCha20-Poly1305",
      iv: new Uint8Array(12),
    };
    const wrapped = await crypto.subtle.wrapKey(
      "raw-secret",
      target,
      wrappingKey,
      algorithm
    );
    const unwrapped = await crypto.subtle.unwrapKey(
      "raw-secret",
      wrapped,
      wrappingKey,
      algorithm,
      "AES-GCM",
      true,
      ["encrypt"]
    );
    expect(await crypto.subtle.exportKey("raw", unwrapped)).toEqual(
      await crypto.subtle.exportKey("raw", target)
    );
  });
});

describe("Modern WebCrypto ML-DSA", () => {
  it("signs and verifies with every parameter set and context", async () => {
    for (const name of ["ML-DSA-44", "ML-DSA-65", "ML-DSA-87"] as const) {
      const keyPair = await crypto.subtle.generateKey(name, false, [
        "sign",
        "verify",
      ]);
      const algorithm = {
        name,
        context: new TextEncoder().encode("llrt"),
      };
      const signature = await crypto.subtle.sign(
        algorithm,
        keyPair.privateKey,
        DATA
      );
      const secondSignature = await crypto.subtle.sign(
        algorithm,
        keyPair.privateKey,
        DATA
      );
      expect(new Uint8Array(secondSignature)).not.toEqual(
        new Uint8Array(signature)
      );
      expect(
        await crypto.subtle.verify(
          algorithm,
          keyPair.publicKey,
          signature,
          DATA
        )
      ).toBe(true);
      expect(
        await crypto.subtle.verify(
          { name, context: new Uint8Array() },
          keyPair.publicKey,
          signature,
          DATA
        )
      ).toBe(false);
    }
  });

  it("round-trips public keys and private seeds", async () => {
    const name = "ML-DSA-44";
    const keyPair = await crypto.subtle.generateKey(name, true, [
      "sign",
      "verify",
    ]);
    const signature = await crypto.subtle.sign(name, keyPair.privateKey, DATA);

    for (const format of ["raw-public", "spki"] as const) {
      const encoded = await crypto.subtle.exportKey(format, keyPair.publicKey);
      const imported = await crypto.subtle.importKey(
        format,
        encoded,
        name,
        true,
        ["verify"]
      );
      expect(await crypto.subtle.verify(name, imported, signature, DATA)).toBe(
        true
      );
    }

    for (const format of ["raw-seed", "pkcs8"] as const) {
      const encoded = await crypto.subtle.exportKey(format, keyPair.privateKey);
      const imported = await crypto.subtle.importKey(
        format,
        encoded,
        name,
        true,
        ["sign"]
      );
      const importedSignature = await crypto.subtle.sign(name, imported, DATA);
      expect(
        await crypto.subtle.verify(
          name,
          keyPair.publicKey,
          importedSignature,
          DATA
        )
      ).toBe(true);
    }

    for (const key of [keyPair.publicKey, keyPair.privateKey]) {
      const jwk = await crypto.subtle.exportKey("jwk", key);
      expect(jwk).toMatchObject({ kty: "AKP", alg: name });
      const imported = await crypto.subtle.importKey(
        "jwk",
        jwk,
        name,
        true,
        key.type === "private" ? ["sign"] : ["verify"]
      );
      expect(imported.type).toBe(key.type);
    }
  });

  it("rejects contexts longer than 255 bytes", async () => {
    const keyPair = await crypto.subtle.generateKey("ML-DSA-44", false, [
      "sign",
    ]);
    await expect(
      crypto.subtle.sign(
        { name: "ML-DSA-44", context: new Uint8Array(256) },
        keyPair.privateKey,
        DATA
      )
    ).rejects.toHaveProperty("name", "OperationError");
  });
});

describe("Modern WebCrypto ML-KEM", () => {
  it("encapsulates and decapsulates bits for every parameter set", async () => {
    const variants = [
      ["ML-KEM-512", 768],
      ["ML-KEM-768", 1088],
      ["ML-KEM-1024", 1568],
    ] as const;

    for (const [name, ciphertextLength] of variants) {
      const keyPair = await crypto.subtle.generateKey(name, true, [
        "decapsulateBits",
        "encapsulateBits",
      ]);
      const encapsulated = await crypto.subtle.encapsulateBits(
        name,
        keyPair.publicKey
      );
      expect(encapsulated.ciphertext.byteLength).toBe(ciphertextLength);
      expect(encapsulated.sharedKey.byteLength).toBe(32);
      expect(
        await crypto.subtle.decapsulateBits(
          name,
          keyPair.privateKey,
          encapsulated.ciphertext
        )
      ).toEqual(encapsulated.sharedKey);
    }
  });

  it("encapsulates and decapsulates CryptoKeys", async () => {
    const keyPair = await crypto.subtle.generateKey("ML-KEM-768", false, [
      "encapsulateKey",
      "decapsulateKey",
    ]);
    const encapsulated = await crypto.subtle.encapsulateKey(
      "ML-KEM-768",
      keyPair.publicKey,
      { name: "AES-GCM", length: 256 },
      true,
      ["encrypt"]
    );
    const decapsulated = await crypto.subtle.decapsulateKey(
      "ML-KEM-768",
      keyPair.privateKey,
      encapsulated.ciphertext,
      { name: "AES-GCM", length: 256 },
      true,
      ["encrypt"]
    );
    expect(await crypto.subtle.exportKey("raw", decapsulated)).toEqual(
      await crypto.subtle.exportKey("raw", encapsulated.sharedKey)
    );
  });

  it("imports the actual shared secret into HMAC keys", async () => {
    const keyPair = await crypto.subtle.generateKey("ML-KEM-768", false, [
      "encapsulateKey",
      "decapsulateBits",
      "decapsulateKey",
    ]);
    let nameReads = 0;
    const hmacAlgorithm = {
      get name() {
        nameReads++;
        return nameReads === 1 ? "HMAC" : "not-an-algorithm";
      },
      hash: "SHA-256",
      length: 256,
    };

    const encapsulated = await crypto.subtle.encapsulateKey(
      "ML-KEM-768",
      keyPair.publicKey,
      hmacAlgorithm,
      true,
      ["sign"]
    );
    expect(nameReads).toBe(1);

    const expected = new Uint8Array(
      await crypto.subtle.decapsulateBits(
        "ML-KEM-768",
        keyPair.privateKey,
        encapsulated.ciphertext
      )
    );
    expect(
      new Uint8Array(
        await crypto.subtle.exportKey("raw-secret", encapsulated.sharedKey)
      )
    ).toEqual(expected);

    nameReads = 0;
    const decapsulated = await crypto.subtle.decapsulateKey(
      "ML-KEM-768",
      keyPair.privateKey,
      encapsulated.ciphertext,
      hmacAlgorithm,
      true,
      ["sign"]
    );
    expect(nameReads).toBe(1);
    expect(
      new Uint8Array(await crypto.subtle.exportKey("raw-secret", decapsulated))
    ).toEqual(expected);
  });

  it("defers shared-key import validation until after KEM operations", async () => {
    const name = "ML-KEM-512";
    const bitOnly = await crypto.subtle.generateKey(name, false, [
      "encapsulateBits",
      "decapsulateBits",
    ]);
    const bitEncapsulation = await crypto.subtle.encapsulateBits(
      name,
      bitOnly.publicKey
    );

    const full = await crypto.subtle.generateKey(name, false, [
      "encapsulateKey",
      "encapsulateBits",
      "decapsulateKey",
    ]);
    const validCiphertext = (
      await crypto.subtle.encapsulateBits(name, full.publicKey)
    ).ciphertext;

    const cases = [
      {
        algorithm: { name: "HMAC", hash: "SHA-256", length: 0 },
        usages: ["sign"] as webcrypto.KeyUsage[],
        importError: "DataError",
      },
      {
        algorithm: { name: "HMAC", hash: "SHA-256" },
        usages: ["encrypt"] as webcrypto.KeyUsage[],
        importError: "SyntaxError",
      },
    ];

    for (const { algorithm, usages, importError } of cases) {
      await expect(
        crypto.subtle.encapsulateKey(
          name,
          bitOnly.publicKey,
          algorithm,
          false,
          usages
        )
      ).rejects.toHaveProperty("name", "InvalidAccessError");

      await expect(
        crypto.subtle.decapsulateKey(
          name,
          bitOnly.privateKey,
          bitEncapsulation.ciphertext,
          algorithm,
          false,
          usages
        )
      ).rejects.toHaveProperty("name", "InvalidAccessError");

      await expect(
        crypto.subtle.decapsulateKey(
          name,
          full.privateKey,
          new Uint8Array(767),
          algorithm,
          false,
          usages
        )
      ).rejects.toHaveProperty("name", "OperationError");

      await expect(
        crypto.subtle.encapsulateKey(
          name,
          full.publicKey,
          algorithm,
          false,
          usages
        )
      ).rejects.toHaveProperty("name", importError);

      await expect(
        crypto.subtle.decapsulateKey(
          name,
          full.privateKey,
          validCiphertext,
          algorithm,
          false,
          usages
        )
      ).rejects.toHaveProperty("name", importError);
    }
  }, 30000);

  it("assigns KEM usages canonically", async () => {
    const keyPair = await crypto.subtle.generateKey("ML-KEM-512", false, [
      "decapsulateBits",
      "encapsulateKey",
      "encapsulateBits",
      "decapsulateKey",
      "encapsulateKey",
    ]);
    expect(keyPair.publicKey.usages).toEqual([
      "encapsulateKey",
      "encapsulateBits",
    ]);
    expect(keyPair.privateKey.usages).toEqual([
      "decapsulateKey",
      "decapsulateBits",
    ]);
  });

  it("round-trips public keys and private seeds", async () => {
    const name = "ML-KEM-512";
    const keyPair = await crypto.subtle.generateKey(name, true, [
      "encapsulateBits",
      "decapsulateBits",
    ]);

    for (const [key, formats, usages] of [
      [keyPair.publicKey, ["raw-public", "spki", "jwk"], ["encapsulateBits"]],
      [keyPair.privateKey, ["raw-seed", "pkcs8", "jwk"], ["decapsulateBits"]],
    ] as const) {
      for (const format of formats) {
        const encoded = await crypto.subtle.exportKey(format, key);
        const imported = await crypto.subtle.importKey(
          format,
          encoded,
          name,
          true,
          usages
        );
        expect(imported.type).toBe(key.type);
      }
    }
  });

  it("rejects malformed ciphertext", async () => {
    const keyPair = await crypto.subtle.generateKey("ML-KEM-512", false, [
      "decapsulateBits",
    ]);
    await expect(
      crypto.subtle.decapsulateBits(
        "ML-KEM-512",
        keyPair.privateKey,
        new Uint8Array(767)
      )
    ).rejects.toHaveProperty("name", "OperationError");
  });
});
