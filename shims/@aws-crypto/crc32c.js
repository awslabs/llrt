import { Crc32c as CryptoCrc32c } from "node:crypto";

export const Crc32c = CryptoCrc32c;

export class AwsCrc32c {
  #crc32c = new CryptoCrc32c();

  update(toHash) {
    if (isEmptyData(toHash)) return;
    this.#crc32c.update(toHash);
  }

  async digest() {
    return numToUint8(this.#crc32c.digest());
  }

  reset() {
    this.#crc32c = new CryptoCrc32c();
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
