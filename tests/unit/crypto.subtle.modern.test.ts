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
