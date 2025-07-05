import { toBase64 } from "@smithy/util-base64";
import { toHex } from "@smithy/util-hex-encoding";
import { toUtf8 } from "@smithy/util-utf8";

const transformToWebStream = () => {
  throw new Error("WebStream is not available for LLRT");
};

async function transformToByteArray() {
  return this.bytes();
}

async function transformToString(encoding) {
  const blob = await this.bytes();
  if (encoding === "base64") {
    return toBase64(blob);
  } else if (encoding === "hex") {
    return toHex(blob);
  }
  return toUtf8(blob);
}

export const sdkStreamMixin = (stream) =>
  Object.assign(stream, {
    transformToByteArray,
    transformToString: transformToString,
    transformToWebStream,
  });
