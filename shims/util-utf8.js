const DECODER = new TextDecoder();
const ENCODER = new TextEncoder();

export const fromUtf8 = (input) => ENCODER.encode(input);
export const toUtf8 = (input) => DECODER.decode(input);

export const toUint8Array = (data) => {
  if (typeof data === "string") {
    return fromUtf8(data);
  }
  if (ArrayBuffer.isView(data)) {
    return new Uint8Array(
      data.buffer,
      data.byteOffset,
      data.byteLength / Uint8Array.BYTES_PER_ELEMENT
    );
  }
  return new Uint8Array(data);
};
