import { toBase64 } from "@smithy/util-base64";
import { toHex } from "@smithy/util-hex-encoding";
import { toUtf8 } from "@smithy/util-utf8";

const transformToWebStream = () => {
  throw new Error("WebStream is not available for LLRT");
};

async function transformToByteArray() {
  return this;
}

async function transformToString(encoding) {
  if (encoding === "base64") {
    return toBase64(this);
  } else if (encoding === "hex") {
    return toHex(this);
  }
  return toUtf8(this);
}

export const sdkStreamMixin = (stream) =>
  Object.assign(stream, {
    transformToByteArray,
    transformToString,
    transformToWebStream,
  });
