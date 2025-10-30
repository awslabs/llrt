/**
 * HTTPS is the HTTP protocol over TLS/SSL.
 */
declare module "https" {
  import { Buffer } from "buffer";

  interface AgentOptions {
    /**
     * Optionally override the trusted CA certificates. Default is to trust
     * the well-known CAs curated by Mozilla. Mozilla's CAs are completely
     * replaced when CAs are explicitly specified using this option.
     */
    ca?: string | Buffer | Array<string | Buffer> | undefined;
    /**
     * Whether to reject unauthorized certificates. Default is `true`.
     */
    rejectUnauthorized?: boolean | undefined;
  }
  /**
   * An `Agent` is responsible for managing connection persistence
   * and reuse for HTTP clients. Currently only a small subset of the options
   * are supported.
   */
  class Agent {
    constructor(opts?: AgentOptions);
  }
}
