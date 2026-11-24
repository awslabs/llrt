// The SDK's fetch handler calls fetch(new Request(...)) with no init, so a
// fetch-level option has to travel on the Request. requestInit is the supported
// Smithy hook for adding options to that Request.
import { FetchHttpHandler as RealFetchHttpHandler } from "../../node_modules/@smithy/fetch-http-handler/dist-es/fetch-http-handler.js";

export * from "../../node_modules/@smithy/fetch-http-handler/dist-es/index.js";

const withRawBody = (options) => {
  const requestInit = options?.requestInit;
  return {
    ...options,
    requestInit(httpRequest) {
      const init =
        typeof requestInit === "function" ? requestInit(httpRequest) : {};
      return { ...init, decompress: false };
    },
  };
};

export class FetchHttpHandler extends RealFetchHttpHandler {
  constructor(options) {
    super(
      typeof options === "function"
        ? async () => withRawBody((await options()) ?? {})
        : withRawBody(options ?? {})
    );
  }

  static create(instanceOrOptions) {
    if (typeof instanceOrOptions?.handle === "function") {
      return instanceOrOptions;
    }
    return new FetchHttpHandler(instanceOrOptions);
  }
}
