import { toUint8Array } from "@smithy/util-utf8";
export const stringHasher = async (checksumAlgorithmFn, body) => {
  const hash = new checksumAlgorithmFn();

  if (body instanceof Blob) {
    const arrayBuffer = await body.arrayBuffer();
    hash.update(arrayBuffer);
  } else {
    hash.update(toUint8Array(body || ""));
  }

  return hash.digest();
};
