import { Crc32c as CryptoCrc32c } from "crypto";
import { isEmptyData, numToUint8 } from "@aws-crypto/util";

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
