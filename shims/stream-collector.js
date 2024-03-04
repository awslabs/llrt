export const streamCollector = async (stream) =>
  new Uint8Array(await stream.arrayBuffer());
