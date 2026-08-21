import defaultImport from "node:zlib";
import legacyImport from "zlib";

const data = "Hello LLRT!!";

it("node:zlib should be the same as zlib", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const {
  deflate,
  inflate,
  deflateSync,
  inflateSync,
  deflateRaw,
  inflateRaw,
  deflateRawSync,
  inflateRawSync,
  gzip,
  gunzip,
  gzipSync,
  gunzipSync,
  brotliCompress,
  brotliDecompress,
  brotliCompressSync,
  brotliDecompressSync,
  zstdCompress,
  zstdDecompress,
  zstdCompressSync,
  zstdDecompressSync,
} = defaultImport;

describe("deflate/inflate", () => {
  it("deflate/inflate", (done) => {
    deflate(data, (err, compressed) => {
      inflate(compressed, (err, decompressed) => {
        expect(data).toEqual(decompressed.toString());
        done();
      });
    });
  });
  it("deflateSync/inflateSync", () => {
    const compressed = deflateSync(data);
    const decompressed = inflateSync(compressed);
    expect(data).toEqual(decompressed.toString());
  });
});

describe("deflateRaw/inflateRaw", () => {
  it("deflateRaw/inflateRaw", (done) => {
    deflateRaw(data, (err, compressed) => {
      inflateRaw(compressed, (err, decompressed) => {
        expect(data).toEqual(decompressed.toString());
        done();
      });
    });
  });
  it("deflateRawSync/inflateRawSync", () => {
    const compressed = deflateRawSync(data);
    const decompressed = inflateRawSync(compressed);
    expect(data).toEqual(decompressed.toString());
  });
});

describe("gzip/gunzip", () => {
  it("gzip/gunzip", (done) => {
    gzip(data, (err, compressed) => {
      gunzip(compressed, (err, decompressed) => {
        expect(data).toEqual(decompressed.toString());
        done();
      });
    });
  });
  it("gzipSync/gunzipSync", () => {
    const compressed = gzipSync(data);
    const decompressed = gunzipSync(compressed);
    expect(data).toEqual(decompressed.toString());
  });
});

describe("brotli", () => {
  it("brotliCompress/brotliDecompress", (done) => {
    brotliCompress(data, (err, compressed) => {
      brotliDecompress(compressed, (err, decompressed) => {
        expect(data).toEqual(decompressed.toString());
        done();
      });
    });
  });
  it("brotliCompressSync/brotliDecompressSync", () => {
    const compressed = brotliCompressSync(data);
    const decompressed = brotliDecompressSync(compressed);
    expect(data).toEqual(decompressed.toString());
  });
});

describe("zstandard", () => {
  it("zstdCompress/zstdDecompress", (done) => {
    zstdCompress(data, (err, compressed) => {
      zstdDecompress(compressed, (err, decompressed) => {
        expect(data).toEqual(decompressed.toString());
        done();
      });
    });
  });
  it("zstdCompressSync/zstdDecompressSync", () => {
    const compressed = zstdCompressSync(data);
    const decompressed = zstdDecompressSync(compressed);
    expect(data).toEqual(decompressed.toString());
  });
});

describe("maxOutputLength", () => {
  // A highly compressible payload: the compressed form is a few bytes but
  // expands well past the limits used below.
  const large = "a".repeat(1024);

  it("should throw when brotliDecompressSync output exceeds the limit", () => {
    const compressed = brotliCompressSync(large);
    expect(() =>
      brotliDecompressSync(compressed, { maxOutputLength: 10 })
    ).toThrow("Cannot create a Buffer larger than 10 bytes");
  });

  it("should pass an error to brotliDecompress when output exceeds the limit", (done) => {
    const compressed = brotliCompressSync(large);
    brotliDecompress(
      compressed,
      { maxOutputLength: 10 },
      (err, decompressed) => {
        expect(err).toBeTruthy();
        expect(err.message).toEqual(
          "Cannot create a Buffer larger than 10 bytes"
        );
        expect(decompressed).toBeUndefined();
        done();
      }
    );
  });

  it("should allow output that exactly matches the limit", () => {
    const compressed = brotliCompressSync(large);
    const decompressed = brotliDecompressSync(compressed, {
      maxOutputLength: large.length,
    });
    expect(decompressed.toString()).toEqual(large);
  });

  it("should ignore a limit larger than the output", () => {
    const compressed = brotliCompressSync(large);
    const decompressed = brotliDecompressSync(compressed, {
      maxOutputLength: large.length + 1,
    });
    expect(decompressed.toString()).toEqual(large);
  });

  it("should apply the limit to gunzipSync", () => {
    const compressed = gzipSync(large);
    expect(() => gunzipSync(compressed, { maxOutputLength: 10 })).toThrow(
      "Cannot create a Buffer larger than 10 bytes"
    );
    expect(
      gunzipSync(compressed, { maxOutputLength: large.length }).toString()
    ).toEqual(large);
  });

  it("should apply the limit to inflateSync", () => {
    const compressed = deflateSync(large);
    expect(() => inflateSync(compressed, { maxOutputLength: 10 })).toThrow(
      "Cannot create a Buffer larger than 10 bytes"
    );
  });

  it("should apply the limit to zstdDecompressSync", () => {
    const compressed = zstdCompressSync(large);
    expect(() =>
      zstdDecompressSync(compressed, { maxOutputLength: 10 })
    ).toThrow("Cannot create a Buffer larger than 10 bytes");
    expect(
      zstdDecompressSync(compressed, {
        maxOutputLength: large.length,
      }).toString()
    ).toEqual(large);
  });

  it("should apply the limit to compression methods", () => {
    expect(() => brotliCompressSync(large, { maxOutputLength: 1 })).toThrow(
      "Cannot create a Buffer larger than 1 bytes"
    );
  });

  it("should still honour other options alongside the limit", () => {
    const compressed = deflateSync(large, { level: 9 });
    expect(
      inflateSync(compressed, { maxOutputLength: large.length }).toString()
    ).toEqual(large);
  });
});
