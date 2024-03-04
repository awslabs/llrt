import { Uint8ArrayBlobAdapter } from "@smithy/util-stream";
export const collectBody = async (streamBody) =>
  Uint8ArrayBlobAdapter.mutate(
    streamBody instanceof Uint8Array
      ? streamBody
      : new Uint8Array(await streamBody.arrayBuffer())
  );
