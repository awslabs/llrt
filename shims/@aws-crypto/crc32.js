import { Crc32 as CryptoCrc32 } from "crypto";

export const Crc32 = CryptoCrc32;

export class AwsCrc32 {
  #crc32 = new CryptoCrc32();

  update(toHash) {
    if (isEmptyData(toHash)) return;
    this.#crc32.update(toHash);
  }

  async digest() {
    return numToUint8(this.#crc32.digest());
  }

  reset() {
    this.#crc32 = new CryptoCrc32();
  }
}

function isEmptyData(data) {
  if (typeof data === "string") {
    return data.length === 0;
  }
  return data.byteLength === 0;
}

function numToUint8(num) {
  return new Uint8Array([
    (num & 0xff000000) >> 24,
    (num & 0x00ff0000) >> 16,
    (num & 0x0000ff00) >> 8,
    num & 0x000000ff,
  ]);
}
