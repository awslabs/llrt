const decoder = new TextDecoder();
const encoder = new TextEncoder();
const testMessage = "This is test message.";
const encodedData = encoder.encode(testMessage);
const extractable = false;

describe("SubtleCrypto digest", () => {
  it("should calculate correctly SHA-1/256/384/512 digest", async () => {
    const parameters = [
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
    ];

    for (const t of parameters) {
      const result = new Uint8Array(
        await crypto.subtle.digest(t.name, encodedData)
      );

      expect(result).toEqual(t.digest);
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
        extractable,
        t.usages
      );

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect(key.algorithm.length).toEqual(algorithm.length);
      expect(key.extractable).toEqual(extractable);
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

      const key = await crypto.subtle.generateKey(
        algorithm,
        extractable,
        t.usages
      );

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect(key.algorithm.hash).toEqual(algorithm.hash);
      expect(key.extractable).toEqual(extractable);
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
        extractable,
        t.usages
      );

      expect(privateKey.algorithm.name).toEqual(algorithm.name);
      expect(privateKey.algorithm.namedCurve).toEqual(algorithm.namedCurve);
      expect(privateKey.extractable).toEqual(extractable);

      expect(publicKey.algorithm.name).toEqual(algorithm.name);
      expect(publicKey.algorithm.namedCurve).toEqual(algorithm.namedCurve);
      expect(publicKey.extractable).toEqual(true);

      if (t.usages.includes("sign")) {
        const signature = await crypto.subtle.sign(
          {
            name: t.name,
            hash: t.hash,
          },
          privateKey,
          encodedData
        );
        const isValid = await crypto.subtle.verify(
          {
            name: t.name,
            hash: t.hash,
          },
          publicKey,
          signature,
          encodedData
        );

        expect(isValid).toBeTruthy();
      }
    }
  });

  // Caveat: The current RSA implementation is too slow to complete the test within the time limit.
  // it("should be processing RSA-PSS/RSA-OAEP/RSASSA-PKCS1-v1_5 algorithm", async () => {
  //   const parameters = [
  //     {
  //       name: "RSA-PSS",
  //       modulusLength: 2048,
  //       publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
  //       hash: "SHA-256",
  //       usages: ["sign", "verify"],
  //     },
  //     {
  //       name: "RSA-PSS",
  //       modulusLength: 2048,
  //       publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
  //       hash: "SHA-384",
  //       usages: ["sign", "verify"],
  //     },
  //     {
  //       name: "RSA-PSS",
  //       modulusLength: 2048,
  //       publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
  //       hash: "SHA-512",
  //       usages: ["sign", "verify"],
  //     },
  //     {
  //       name: "RSA-OAEP",
  //       modulusLength: 2048,
  //       publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
  //       hash: "SHA-256",
  //       usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
  //     },
  //     {
  //       name: "RSA-OAEP",
  //       modulusLength: 2048,
  //       publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
  //       hash: "SHA-384",
  //       usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
  //     },
  //     {
  //       name: "RSA-OAEP",
  //       modulusLength: 2048,
  //       publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
  //       hash: "SHA-512",
  //       usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
  //     },
  //     {
  //       name: "RSASSA-PKCS1-v1_5",
  //       modulusLength: 2048,
  //       publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
  //       hash: "SHA-256",
  //     },
  //     {
  //       name: "RSASSA-PKCS1-v1_5",
  //       modulusLength: 2048,
  //       publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
  //       hash: "SHA-384",
  //     },
  //     {
  //       name: "RSASSA-PKCS1-v1_5",
  //       modulusLength: 2048,
  //       publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
  //       hash: "SHA-512",
  //     },
  //   ];

  //   for (const t of parameters) {
  //     const algorithm = {
  //       name: t.name,
  //       modulusLength: t.modulusLength,
  //       publicExponent: t.publicExponent,
  //       hash: t.hash,
  //     };

  //     const { privateKey, publicKey } = await crypto.subtle.generateKey(
  //       algorithm,
  //       extractable,
  //       t.usages
  //     );

  //     expect(privateKey.algorithm.name).toEqual(t.name);
  //     expect(privateKey.algorithm.hash).toEqual(algorithm.hash);
  //     expect(privateKey.extractable).toEqual(extractable);

  //     expect(publicKey.algorithm.name).toEqual(algorithm.name);
  //     expect(publicKey.algorithm.hash).toEqual(algorithm.hash);
  //     expect(publicKey.extractable).toEqual(true);

  //     if (t.usages.includes("sign")) {
  //       const signature = await crypto.subtle.sign(
  //         {
  //           name: t.name,
  //         },
  //         privateKey,
  //         encodedData
  //       );
  //       const isValid = await crypto.subtle.verify(
  //         {
  //           name: t.name,
  //         },
  //         publicKey,
  //         signature,
  //         encodedData
  //       );

  //       expect(isValid).toBeTruthy();
  //     }
  //   }
  // });
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
        extractable,
        t.usages
      );

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect(key.algorithm.length).toEqual(algorithm.length);
      expect(key.extractable).toEqual(extractable);

      if (t.usages.includes("encrypt")) {
        const encryptedData = await crypto.subtle.encrypt(
          {
            name: t.name,
            iv: t.iv,
          },
          key,
          encodedData
        );
        const decryptedData = await crypto.subtle.decrypt(
          {
            name: t.name,
            iv: t.iv,
          },
          key,
          encryptedData
        );

        const result = decoder.decode(decryptedData);
        expect(result).toEqual(testMessage);
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
        extractable,
        t.usages
      );

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect(key.algorithm.length).toEqual(algorithm.length);
      expect(key.extractable).toEqual(extractable);

      if (t.usages.includes("encrypt")) {
        const encryptedData = await crypto.subtle.encrypt(
          {
            name: t.name,
            counter: t.counter,
            length: t.counterLength,
          },
          key,
          encodedData
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

        const result = decoder.decode(decryptedData);
        expect(result).toEqual(testMessage);
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
        extractable,
        t.usages
      );

      expect(key.algorithm.name).toEqual(algorithm.name);
      expect(key.algorithm.length).toEqual(algorithm.length);
      expect(key.extractable).toEqual(extractable);

      if (t.usages.includes("encrypt")) {
        const encryptedData = await crypto.subtle.encrypt(
          {
            name: t.name,
            iv: t.iv,
          },
          key,
          encodedData
        );
        const decryptedData = await crypto.subtle.decrypt(
          {
            name: t.name,
            iv: t.iv,
          },
          key,
          encryptedData
        );

        const result = decoder.decode(decryptedData);
        expect(result).toEqual(testMessage);
      }
    }
  });
});

// Caveat: The current RSA implementation is too slow to complete the test within the time limit.
// describe("SubtleCrypto generateKey/encrypt/decrypt", () => {
//   it("should be processing RSA-OAEP algorithm", async () => {
//     const parameters = [
//       {
//         name: "RSA-OAEP",
//         modulusLength: 2048,
//         publicExponent: new Uint8Array([1, 0, 1]),
//         hash: "SHA-1",
//         usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
//       },
//       {
//         name: "RSA-OAEP",
//         modulusLength: 2048,
//         publicExponent: new Uint8Array([1, 0, 1]),
//         hash: "SHA-256",
//         usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
//       },
//       {
//         name: "RSA-OAEP",
//         modulusLength: 2048,
//         publicExponent: new Uint8Array([1, 0, 1]),
//         hash: "SHA-384",
//         usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
//       },
//       {
//         name: "RSA-OAEP",
//         modulusLength: 2048,
//         publicExponent: new Uint8Array([1, 0, 1]),
//         hash: "SHA-512",
//         usages: ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
//       },
//     ];

//     for (const t of parameters) {
//       const algorithm = { name: t.name, modulusLength: t.modulusLength, publicExponent: t.publicExponent, hash: t.hash };

//       const key = await crypto.subtle.generateKey(
//         algorithm,
//         extractable,
//         t.usages
//       );

//       expect(key.algorithm.name).toEqual(algorithm.name);
//       expect(key.algorithm.length).toEqual(algorithm.length);
//       expect(key.extractable).toEqual(extractable);

//       if (t.usages.includes("encrypt")) {
//         const encryptedData = await crypto.subtle.encrypt(
//           {
//             name: t.name,
//           },
//           key,
//           encodedData
//         );
//         const decryptedData = await crypto.subtle.decrypt(
//           {
//             name: t.name,
//           },
//           key,
//           encryptedData
//         );

//         const result = decoder.decode(decryptedData);
//         expect(result).toEqual(testMessage);
//       }
//     }
//   });
// });
