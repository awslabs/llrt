import { createHash } from "node:crypto";
import type { webcrypto } from "node:crypto";

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
