import { Crc32 as CryCrc32 } from "crypto";
export const Crc32 = (data) => new CryCrc32().update(data).digest();
export const AwsCrc32 = Crc32;
