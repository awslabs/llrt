import { Crc32c as CryCrc32c } from "crypto";
export const Crc32c = (data) => new CryCrc32c().update(data).digest();
export const AwsCrc32c = Crc32c;
