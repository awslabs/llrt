describe("SubtleCrypto digest", () => {
  const digestMessage = "This is test message.";
  const encoder = new TextEncoder();

  it("should calculate correctly SHA-1 digest", async () => {
    const digestSha1 = new Uint8Array([
      77, 178, 99, 24, 75, 24, 35, 67, 75, 116, 194, 145, 251, 77, 201, 158,
      163, 128, 52, 146,
    ]);
    const data = encoder.encode(digestMessage);
    const result = new Uint8Array(await crypto.subtle.digest("SHA-1", data));
    expect(result).toEqual(digestSha1);
  });

  it("should calculate correctly SHA-256 digest", async () => {
    const digestSha256 = new Uint8Array([
      25, 251, 197, 98, 46, 227, 66, 238, 69, 151, 67, 175, 68, 184, 76, 182,
      55, 172, 65, 183, 49, 68, 7, 196, 44, 100, 140, 80, 173, 34, 85, 162,
    ]);
    const data = encoder.encode(digestMessage);
    const result = new Uint8Array(await crypto.subtle.digest("SHA-256", data));
    expect(result).toEqual(digestSha256);
  });

  it("should calculate correctly SHA-384 digest", async () => {
    const digestSha384 = new Uint8Array([
      120, 197, 163, 25, 168, 231, 230, 188, 103, 31, 71, 249, 169, 230, 153,
      177, 79, 131, 234, 88, 93, 74, 22, 71, 169, 225, 35, 40, 129, 238, 168,
      107, 148, 105, 238, 23, 160, 190, 147, 195, 162, 135, 202, 230, 26, 130,
      124, 245,
    ]);
    const data = encoder.encode(digestMessage);
    const result = new Uint8Array(await crypto.subtle.digest("SHA-384", data));
    expect(result).toEqual(digestSha384);
  });

  it("should calculate correctly SHA-512 digest", async () => {
    const digestSha512 = new Uint8Array([
      197, 205, 134, 50, 153, 136, 53, 103, 159, 209, 236, 27, 212, 147, 229,
      162, 64, 1, 116, 206, 59, 187, 12, 223, 135, 121, 147, 143, 26, 203, 161,
      238, 57, 27, 254, 202, 96, 207, 172, 168, 12, 47, 150, 164, 182, 235, 183,
      159, 134, 226, 198, 183, 61, 128, 211, 133, 33, 12, 168, 19, 139, 120, 80,
      145,
    ]);
    const data = encoder.encode(digestMessage);
    const result = new Uint8Array(await crypto.subtle.digest("SHA-512", data));
    expect(result).toEqual(digestSha512);
  });

  it("should throw an error for SHA-0 digest", async () => {
    const data = encoder.encode(digestMessage);
    await expect(crypto.subtle.digest("SHA-0", data)).rejects.toThrow(
      "'SHA-0' not available"
    );
  });
});
