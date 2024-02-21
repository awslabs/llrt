import { Uint8ArrayBlobAdapter } from "@smithy/util-stream";
export const collectBody = async (streamBody) =>
  Uint8ArrayBlobAdapter.mutate(await streamBody.typedArray());
