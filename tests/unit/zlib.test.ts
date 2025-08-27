import defaultImport from "node:zlib";
import legacyImport from "zlib";
import * as legacyNamedImport from "zlib";

const data = "Hello LLRT!!";

const modules = {
  "node:zlib": defaultImport,
  zlib: legacyImport,
  "* as zlib": legacyNamedImport,
};
for (const module in modules) {
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
  } = modules[module];

  describe(module, () => {
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
  });
}
