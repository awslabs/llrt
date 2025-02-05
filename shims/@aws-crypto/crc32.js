import { Crc32 as CryptoCrc32 } from "crypto";
import { isEmptyData, numToUint8 } from "@aws-crypto/util";

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
