import { Sha1 as CrySha1 } from "crypto";
export const Sha1 = (data) => new CrySha1().update(data).digest();
