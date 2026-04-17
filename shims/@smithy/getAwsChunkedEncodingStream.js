// Shim for @smithy/util-stream getAwsChunkedEncodingStream (browser variant).
// Fixes vs upstream: (1) encode to Uint8Array (LLRT fetch requires BufferSource),
// (2) buffer chunks >= 8192 bytes (S3 minimum chunk size),
// (3) compute checksum inline (avoids tee + streamHasher),
// (4) convert Node.js Readable to ReadableStream via Readable.toWeb().

const enc = new TextEncoder();
const CRLF = enc.encode("\r\n");
const LAST = enc.encode("0\r\n");
const MIN_CHUNK = 8192;

export const getAwsChunkedEncodingStream = (readableStream, options) => {
  if (
    typeof readableStream.getReader !== "function" &&
    typeof readableStream._read === "function"
  ) {
    const { Readable } = require("stream");
    readableStream = Readable.toWeb(readableStream);
  }

  const { base64Encoder, checksumAlgorithmFn, checksumLocationName } = options;
  const hash =
    base64Encoder && checksumAlgorithmFn && checksumLocationName
      ? new checksumAlgorithmFn()
      : null;
  const reader = readableStream.getReader();
  let buf = [],
    bufLen = 0;

  return new ReadableStream({
    async pull(controller) {
      while (bufLen < MIN_CHUNK) {
        const { value, done } = await reader.read();
        if (done) {
          if (bufLen > 0) {
            let merged;
            if (buf.length === 1) {
              merged = buf[0];
            } else {
              merged = new Uint8Array(bufLen);
              let off = 0;
              for (let i = 0; i < buf.length; i++) {
                merged.set(buf[i], off);
                off += buf[i].byteLength;
              }
            }
            buf = [];
            bufLen = 0;
            if (hash) hash.update(merged);
            controller.enqueue(
              enc.encode(merged.byteLength.toString(16) + "\r\n")
            );
            controller.enqueue(merged);
            controller.enqueue(CRLF);
          }
          controller.enqueue(LAST);
          if (hash) {
            controller.enqueue(
              enc.encode(
                checksumLocationName +
                  ":" +
                  base64Encoder(await hash.digest()) +
                  "\r\n\r\n"
              )
            );
          }
          controller.close();
          return;
        }
        const b =
          value instanceof Uint8Array
            ? value
            : enc.encode(typeof value === "string" ? value : String(value));
        buf.push(b);
        bufLen += b.byteLength;
      }
      let merged;
      if (buf.length === 1) {
        merged = buf[0];
      } else {
        merged = new Uint8Array(bufLen);
        let off = 0;
        for (let i = 0; i < buf.length; i++) {
          merged.set(buf[i], off);
          off += buf[i].byteLength;
        }
      }
      buf = [];
      bufLen = 0;
      if (hash) hash.update(merged);
      controller.enqueue(enc.encode(merged.byteLength.toString(16) + "\r\n"));
      controller.enqueue(merged);
      controller.enqueue(CRLF);
    },
  });
};
