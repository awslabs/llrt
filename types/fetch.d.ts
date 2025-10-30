import { Agent } from "https";

export {};

declare global {
  interface BlobOpts {
    /**
     * One of either `'transparent'` or `'native'`. When set to `'native'`, line endings in string source parts
     * will be converted to the platform native line-ending as specified by `import { EOL } from 'os'`.
     */
    endings?: "transparent" | "native";
    /**
     * The Blob content-type. The intent is for `type` to convey the MIME media type of the data,
     * however no validation of the type format is performed.
     */
    type?: string | undefined;
  }

  /**
   * The `Body` of a {@link Response} or {@link Request}.
   * Currently NOT a `ReadableStream`.
   */
  type Body = QuickJS.ArrayBufferView | Blob | null;

  /**
   * A [`Blob`](https://developer.mozilla.org/en-US/docs/Web/API/Blob) encapsulates immutable, raw data.
   */
  class Blob {
    /**
     * Creates a new `Blob` object containing a concatenation of the given sources.
     *
     * {ArrayBuffer}, and {Blob} sources are copied into the 'Blob' and can therefore be
     * safely modified after the 'Blob' is created.
     *
     * String sources are also copied into the `Blob`.
     */
    constructor(parts: Array<ArrayBuffer | string | Blob>, opts?: BlobOpts);
    /**
     * The total size of the `Blob` in bytes.
     */
    readonly size: number;
    /**
     * The content-type of the `Blob`.
     */
    readonly type: string;
    /**
     * Returns a promise that fulfills with an [ArrayBuffer](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/ArrayBuffer) containing a copy of
     * the `Blob` data.
     */
    arrayBuffer(): Promise<ArrayBuffer>;
    /**
     * Creates and returns a new `Blob` containing a subset of this `Blob` objects
     * data. The original `Blob` is not altered.
     * @param start The starting index.
     * @param end The ending index.
     * @param type The content-type for the new `Blob`
     */
    slice(start?: number, end?: number, type?: string): Blob;
    /**
     * Returns a promise that fulfills with the contents of the `Blob` decoded as a UTF-8 string.
     */
    text(): Promise<string>;
    /**
     * Returns a promise that resolves with an Uint8Array containing the contents of the Blob.
     */
    bytes(): Promise<Uint8Array>;
  }

  interface FileOpts extends BlobOpts {
    /**
     * The last modified date of the file as the number of milliseconds since the Unix epoch (January 1, 1970 at midnight).
     * Files without a known last modified date return the current date.
     */
    lastModified?: number;
  }

  class File extends Blob {
    /**
     * Returns a newly constructed File.
     */
    constructor(
      data: Array<ArrayBuffer | string | Blob>,
      fileName: string,
      opts?: FileOpts
    );
    /**
     * Name of the file referenced by the File object.
     */
    readonly name: string;
    /**
     * The last modified date of the file as the number of milliseconds since the Unix epoch (January 1, 1970 at midnight).
     * Files without a known last modified date return the current date.
     */
    readonly lastModified: number;
  }

  type HeadersLike = Record<string, string> | Headers;

  type HeadersOpts = string[][] | HeadersLike;

  class Headers implements Iterable<[string, string]> {
    /**
     * Creates a new Headers object.
     */
    constructor(opts?: HeadersOpts);
    /**
     * Appends a new value onto an existing header inside a Headers object, or adds the header if it does not already exist.
     */
    readonly append: (name: string, value: string) => void;
    /**
     * Deletes a header from a Headers object.
     */
    readonly delete: (name: string) => void;
    /**
     * A String sequence representing the values of the retrieved header or null if this header is not set.
     */
    readonly get: (name: string) => string | null;
    /**
     * Returns a boolean stating whether a Headers object contains a certain header.
     */
    readonly has: (name: string) => boolean;
    /**
     * Sets a new value for an existing header inside a Headers object, or adds the header if it does not already exist.
     */
    readonly set: (name: string, value: string) => void;
    /**
     * Returns an array containing the values of all Set-Cookie headers associated with a response.
     */
    readonly getSetCookie: () => string[];
    /**
     * Executes a provided function once for each key/value pair in this Headers object.
     */
    readonly forEach: (
      callbackfn: (value: string, key: string) => void
    ) => void;
    /**
     * Returns an iterator allowing you to go through all keys of the key/value pairs contained in this object.
     */
    readonly keys: () => IterableIterator<string>;
    /**
     * Returns an iterator allowing you to go through all values of the key/value pairs contained in this object.
     */
    readonly values: () => IterableIterator<string>;
    /**
     * Returns an iterator allowing to go through all key/value pairs contained in this object.
     */
    readonly entries: () => IterableIterator<[string, string]>;
    readonly [Symbol.iterator]: () => Iterator<[string, string]>;
  }

  interface RequestOpts {
    url?: string;
    method?: string;
    signal?: AbortSignal;
    body?: Blob;
    headers?: HeadersLike;
    agent?: Agent;
  }

  type RequestCache = "no-cache";

  type RequestMode = "navigate";

  /**
   * The Request interface of the Fetch API represents a resource request.
   */
  class Request {
    /**
     * Creates a new Request object.
     */
    constructor(input: string | URL | Request, init?: RequestOpts);
    /**
     * Contains the cache mode of the request
     */
    readonly cache: RequestCache;
    /**
     * Contains the associated Headers object of the request.
     */
    readonly headers: Headers;
    /**
     * Contains the request's method (GET, POST, etc.)
     */
    readonly method: string;
    /**
     * Contains the mode of the request
     */
    readonly mode: RequestMode;
    /**
     * Contains the URL of the request.
     */
    readonly url: string;
    /**
     * Contains the request's keepalive setting (true or false), which indicates whether llrt will
     * keep the associated connection alive.
     */
    readonly keepalive: boolean;
    /**
     * Returns the {@link AbortSignal} associated with the request
     */
    readonly signal: AbortSignal;
    /**
     * The body content.
     */
    readonly body: Body;
    /**
     * Stores true or false to indicate whether or not the body has been used in a request yet.
     */
    readonly bodyUsed: boolean;
    /**
     * The {@link Agent} associated with the request.
     */
    readonly agent: Agent;
    /**
     * Returns a promise that resolves with an ArrayBuffer representation of the request body.
     */
    readonly arrayBuffer: () => Promise<ArrayBuffer>;
    /**
     * Returns a promise that resolves with a {@link Blob} representation of the request body.
     */
    readonly blob: () => Promise<Blob>;
    /**
     * Returns a promise that resolves with a {@link Uint8Array} representation of the request body.
     */
    readonly bytes: () => Promise<Uint8Array>;
    /**
     * Returns a promise that resolves with the result of parsing the request body as JSON.
     */
    readonly json: () => Promise<unknown>;
    /**
     * Returns a promise that resolves with a text representation of the request body.
     */
    readonly text: () => Promise<string>;
    /**
     * Creates a copy of the current {@link Request} object.
     */
    readonly clone: () => Request;
  }

  type ResponseType = "basic" | "error";

  interface ResponseInit {
    readonly status?: number;
    readonly statusText?: string;
    readonly headers?: HeadersLike;
  }

  interface ResponseOpts extends ResponseInit {
    readonly url?: string;
    readonly signal?: AbortSignal;
  }

  /**
   * The Response interface of the Fetch API represents the response to a request.
   */
  class Response {
    /**
     * Creates a new Response object.
     */
    constructor(body?: Body, opts?: ResponseOpts);

    /**
     * The {@link Headers} object associated with the response.
     */
    readonly headers: Headers;
    /**
     * A boolean indicating whether the response was successful (status in the range 200 – 299) or not.
     */
    readonly ok: boolean;
    /**
     * The status code of the response. (This will be 200 for a success).
     */
    readonly status: number;
    /**
     * The status message corresponding to the status code. (e.g., OK for 200).
     */
    readonly statusText: string;
    /**
     * The type of the response.
     */
    readonly type: ResponseType;
    readonly url: string;
    /**
     * Indicates whether or not the response is the result of a redirect (that is, its URL list has more than one entry).
     */
    readonly redirected: boolean;
    /**
     * The body content (NOT IMPLEMENTED YET).
     */
    readonly body: undefined;
    /**
     * Stores a boolean value that declares whether the body has been used in a response yet.
     */
    readonly bodyUsed: boolean;
    /**
     * Returns a promise that resolves with an {@link ArrayBuffer} representation of the response body.
     */
    readonly arrayBuffer: () => Promise<ArrayBuffer>;
    /**
     * Returns a promise that resolves with a {@link Blob} representation of the response body.
     */
    readonly blob: () => Promise<Blob>;
    /**
     * Returns a promise that resolves with the result of parsing the response body text as JSON.
     */
    readonly json: () => Promise<unknown>;
    /**
     * Returns a promise that resolves with a text representation of the response body.
     */
    readonly text: () => Promise<string>;
    /**
     * Creates a clone of a {@link Response} object.
     */
    readonly clone: () => Response;
    /**
     * Returns a new {@link Response} object associated with a network error.
     */
    static error(): Response;
    /**
     * Returns a new {@link Response} object for returning the provided JSON encoded data.
     */
    static json(data: any, init?: ResponseInit): Response;
    /**
     * Returns a new {@link Response} with a different URL.
     */
    static redirect(url: string | URL, status?: number): Response;
  }

  function fetch(
    input: string | URL | Request,
    init?: RequestOpts
  ): Promise<Response>;
}
