import { toBase64 } from "@smithy/util-base64";
import { toHex } from "@smithy/util-hex-encoding";
import { toUtf8 } from "@smithy/util-utf8";

const transformToWebStream = () => {
  throw new Error("WebStream is not available for LLRT");
};

async function transformToByteArray() {
  return await this.typedArray();
}

async function transformToString(encoding) {
  const typedArray = await this.typedArray();
  if (encoding === "base64") {
    return toBase64(typedArray);
  } else if (encoding === "hex") {
    return toHex(typedArray);
  }
  return toUtf8(typedArray);
}

export const sdkStreamMixin = (stream) =>
  Object.assign(stream, {
    transformToByteArray,
    transformToString,
    transformToWebStream,
  });
