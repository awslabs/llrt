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
     * Client certificate chain in PEM format, sent to servers that request
     * client authentication (mutual TLS). The chain is the certificate for
     * `key`, followed by any intermediate certificates, in order, without
     * the root CA. Must be set together with `key`. Only one chain is
     * supported, so an array must have a single element.
     */
    cert?: string | Buffer | Array<string | Buffer> | undefined;
    /**
     * Private key for `cert`, in PEM format. Must be set together with
     * `cert`. Only one key is supported, so an array must have a single
     * element. An encrypted key is decrypted with `passphrase`. With the
     * rustls TLS backends, it must be a PKCS#8 key encrypted with PBES2 and
     * AES-CBC, such as `openssl pkcs8 -topk8` writes.
     */
    key?: string | Buffer | Array<string | Buffer> | undefined;
    /**
     * Passphrase for an encrypted `key`.
     */
    passphrase?: string | undefined;
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
