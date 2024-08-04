const zlib = require("zlib");
const data = "Hello LLRT!!";

describe("deflate/inflate", () => {
  it("deflate/inflate", (done) => {
    zlib.deflate(data, (err, compressed) => {
      zlib.inflate(compressed, (err, decompressed) => {
        expect(data).toEqual(decompressed.toString());
        done();
      });
    });
  });
  it("deflateSync/inflateSync", () => {
    const compressed = zlib.deflateSync(data);
    const decompressed = zlib.inflateSync(compressed);
    expect(data).toEqual(decompressed.toString());
  });
});

describe("deflateRaw/inflateRaw", () => {
  it("deflateRaw/inflateRaw", (done) => {
    zlib.deflateRaw(data, (err, compressed) => {
      zlib.inflateRaw(compressed, (err, decompressed) => {
        expect(data).toEqual(decompressed.toString());
        done();
      });
    });
  });
  it("deflateRawSync/inflateRawSync", () => {
    const compressed = zlib.deflateRawSync(data);
    const decompressed = zlib.inflateRawSync(compressed);
    expect(data).toEqual(decompressed.toString());
  });
});

describe("gzip/gunzip", () => {
  it("gzip/gunzip", (done) => {
    zlib.gzip(data, (err, compressed) => {
      zlib.gunzip(compressed, (err, decompressed) => {
        expect(data).toEqual(decompressed.toString());
        done();
      });
    });
  });
  it("gzipSync/gunzipSync", () => {
    const compressed = zlib.gzipSync(data);
    const decompressed = zlib.gunzipSync(compressed);
    expect(data).toEqual(decompressed.toString());
  });
});

describe("brotli", () => {
  it("brotliCompress/brotliDecompress", (done) => {
    zlib.brotliCompress(data, (err, compressed) => {
      zlib.brotliDecompress(compressed, (err, decompressed) => {
        expect(data).toEqual(decompressed.toString());
        done();
      });
    });
  });
  it("brotliCompressSync/brotliDecompressSync", () => {
    const compressed = zlib.brotliCompressSync(data);
    const decompressed = zlib.brotliDecompressSync(compressed);
    expect(data).toEqual(decompressed.toString());
  });
});

describe("zstandard", () => {
  it("zstandardCompress/zstandardDecompress", (done) => {
    zlib.zstandardCompress(data, (err, compressed) => {
      zlib.zstandardDecompress(compressed, (err, decompressed) => {
        expect(data).toEqual(decompressed.toString());
        done();
      });
    });
  });
  it("zstandardCompressSync/zstandardDecompressSync", () => {
    const compressed = zlib.zstandardCompressSync(data);
    const decompressed = zlib.zstandardDecompressSync(compressed);
    expect(data).toEqual(decompressed.toString());
  });
});
