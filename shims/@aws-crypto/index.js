import {
  Sha256,
  Sha1 as CrySha1,
  Crc32 as CryCrc32,
  Crc32c as CryCrc32c,
} from "crypto";
const Crc32c = (data) => new CryCrc32c().update(data).digest();
const Crc32 = (data) => new CryCrc32().update(data).digest();
const Sha1 = (data) => new CrySha1().update(data).digest();
const AwsCrc32 = Crc32;
const AwsCrc32c = Crc32c;
export { Sha256, Crc32c, Crc32, AwsCrc32, AwsCrc32c, Sha1 };
