/**
 * The `tls` module provides an implementation of the Transport Layer Security
 * (TLS) and Secure Socket Layer (SSL) protocols that is built on top of the
 * `net` module.
 *
 * It can be accessed using:
 *
 * ```js
 * import * as tls from 'tls';
 * ```
 */
declare module "tls" {
  import { Buffer } from "buffer";
  import { DefaultDuplexStream as Duplex } from "stream";

  interface ConnectionOptions {
    /**
     * Host the client should connect to.
     * @default 'localhost'
     */
    host?: string | undefined;

    /**
     * Port the client should connect to.
     */
    port?: number | undefined;

    /**
     * Server name for the SNI (Server Name Indication) TLS extension.
     * It is the name of the host being connected to, and must be a host name, and not an IP address.
     */
    servername?: string | undefined;

    /**
     * If `true`, the server certificate is verified against the list of supplied CAs.
     * An `'error'` event is emitted if verification fails.
     * @default true
     */
    rejectUnauthorized?: boolean | undefined;

    /**
     * Optionally override the trusted CA certificates.
     * Default is to trust the well-known CAs curated by Mozilla.
     */
    ca?: string | Buffer | Array<string | Buffer> | undefined;

    /**
     * Private key in PEM format for client certificate authentication.
     */
    key?: string | Buffer | undefined;

    /**
     * Certificate chain in PEM format for client certificate authentication.
     */
    cert?: string | Buffer | undefined;

    /**
     * An array of strings or Buffers of supported ALPN protocols.
     */
    ALPNProtocols?: string[] | undefined;

    /**
     * If `true`, the socket won't automatically send a FIN packet when the
     * other end of the socket sends a FIN packet.
     * @default false
     */
    allowHalfOpen?: boolean | undefined;

    /**
     * Minimum TLS version to allow. One of 'TLSv1.2' or 'TLSv1.3'.
     * @default 'TLSv1.2'
     */
    minVersion?: string | undefined;

    /**
     * Maximum TLS version to allow. One of 'TLSv1.2' or 'TLSv1.3'.
     * @default 'TLSv1.3'
     */
    maxVersion?: string | undefined;

    /**
     * A pre-built SecureContext object to use for the connection.
     * If provided, other certificate options (ca, cert, key) are ignored.
     */
    secureContext?: SecureContext | undefined;
  }

  interface SecureContextOptions {
    /**
     * Private key in PEM format.
     */
    key?: string | Buffer | undefined;

    /**
     * Certificate chain in PEM format.
     */
    cert?: string | Buffer | undefined;

    /**
     * Optionally override the trusted CA certificates.
     */
    ca?: string | Buffer | Array<string | Buffer> | undefined;

    /**
     * Cipher suites to use.
     */
    ciphers?: string | undefined;

    /**
     * Minimum TLS version to allow.
     */
    minVersion?: string | undefined;

    /**
     * Maximum TLS version to allow.
     */
    maxVersion?: string | undefined;
  }

  interface CipherInfo {
    name: string;
    standardName: string;
    version: string;
  }

  interface PeerCertificate {
    subject: { [key: string]: string };
    issuer: { [key: string]: string };
    valid_from: string;
    valid_to: string;
    fingerprint: string;
    fingerprint256: string;
    serialNumber: string;
  }

  type SocketReadyState =
    | "opening"
    | "open"
    | "readOnly"
    | "writeOnly"
    | "closed";

  /**
   * Performs transparent encryption of written data and all required TLS
   * negotiation.
   *
   * Instances of `tls.TLSSocket` implement the duplex `Stream` interface.
   */
  class TLSSocket extends Duplex {
    /**
     * Construct a new `TLSSocket` object from an existing socket.
     */
    constructor(options?: { allowHalfOpen?: boolean });

    /**
     * Returns `true` if the peer certificate was signed by one of the CAs
     * specified when creating the `tls.TLSSocket` instance, otherwise `false`.
     */
    readonly authorized: boolean;

    /**
     * Returns the reason why the peer's certificate was not verified.
     * This property is set only when `tlsSocket.authorized === false`.
     */
    readonly authorizationError?: string;

    /**
     * Always returns `true`. This may be used to distinguish TLS sockets from
     * regular `net.Socket` instances.
     */
    readonly encrypted: boolean;

    /**
     * If `true`, `socket.connect()` was called and has not yet finished.
     * It will stay `true` until the socket becomes connected, then it is set to `false`
     * and the `'secureConnect'` event is emitted.
     */
    readonly connecting: boolean;

    /**
     * This is `true` if the socket is not connected yet.
     */
    readonly pending: boolean;

    /**
     * String containing the selected ALPN protocol.
     */
    readonly alpnProtocol?: string | false | null;

    /**
     * Returns the string representation of the local IP address.
     */
    readonly localAddress?: string;

    /**
     * Returns the numeric representation of the local port.
     */
    readonly localPort?: number;

    /**
     * Returns the string representation of the local IP family.
     */
    readonly localFamily?: string;

    /**
     * Returns the string representation of the remote IP address.
     */
    readonly remoteAddress?: string;

    /**
     * Returns the numeric representation of the remote port.
     */
    readonly remotePort?: number;

    /**
     * Returns the string representation of the remote IP family.
     */
    readonly remoteFamily?: string;

    /**
     * This property represents the state of the connection as a string.
     */
    readonly readyState: SocketReadyState;

    /**
     * Returns the string representation of the server name requested via SNI.
     */
    readonly servername?: string;

    /**
     * Initiate a TLS connection on a given socket.
     *
     * @param options Connection options
     * @param callback Callback to be called when the connection is established
     * @returns The socket itself
     */
    connect(options: ConnectionOptions, callback?: () => void): this;

    /**
     * Returns a string containing the negotiated SSL/TLS protocol version.
     * The value `'unknown'` will be returned for connected sockets that have
     * not completed the handshaking process.
     */
    getProtocol(): string | null;

    /**
     * Returns an object representing the peer's certificate. If the peer does not
     * provide a certificate, an empty object will be returned.
     */
    getPeerCertificate(): PeerCertificate | {};

    /**
     * Returns an object containing information on the negotiated cipher suite.
     */
    getCipher(): CipherInfo | null;

    /**
     * Half-closes the socket.
     *
     * @param callback Optional callback for when the socket is finished.
     * @returns The socket itself.
     */
    end(callback?: () => void): this;

    /**
     * events.EventEmitter
     *   1. close
     *   2. secureConnect
     *   3. data
     *   4. end
     *   5. error
     *   6. keylog
     */
    addListener(event: string, listener: (...args: any[]) => void): this;
    addListener(event: "close", listener: (hadError: boolean) => void): this;
    addListener(event: "secureConnect", listener: () => void): this;
    addListener(event: "data", listener: (data: Buffer) => void): this;
    addListener(event: "drain", listener: () => void): this;
    addListener(event: "end", listener: () => void): this;
    addListener(event: "error", listener: (err: Error) => void): this;
    addListener(event: "keylog", listener: (line: Buffer) => void): this;
    emit(event: string | symbol, ...args: any[]): boolean;
    emit(event: "close", hadError: boolean): boolean;
    emit(event: "secureConnect"): boolean;
    emit(event: "data", data: Buffer): boolean;
    emit(event: "end"): boolean;
    emit(event: "error", err: Error): boolean;
    emit(event: "keylog", line: Buffer): boolean;
    on(event: string, listener: (...args: any[]) => void): this;
    on(event: "close", listener: (hadError: boolean) => void): this;
    on(event: "secureConnect", listener: () => void): this;
    on(event: "data", listener: (data: Buffer) => void): this;
    on(event: "end", listener: () => void): this;
    on(event: "error", listener: (err: Error) => void): this;
    on(event: "keylog", listener: (line: Buffer) => void): this;
    once(event: string, listener: (...args: any[]) => void): this;
    once(event: "close", listener: (hadError: boolean) => void): this;
    once(event: "secureConnect", listener: () => void): this;
    once(event: "data", listener: (data: Buffer) => void): this;
    once(event: "end", listener: () => void): this;
    once(event: "error", listener: (err: Error) => void): this;
    once(event: "keylog", listener: (line: Buffer) => void): this;
    prependListener(event: string, listener: (...args: any[]) => void): this;
    prependListener(
      event: "close",
      listener: (hadError: boolean) => void
    ): this;
    prependListener(event: "secureConnect", listener: () => void): this;
    prependListener(event: "data", listener: (data: Buffer) => void): this;
    prependListener(event: "end", listener: () => void): this;
    prependListener(event: "error", listener: (err: Error) => void): this;
    prependListener(event: "keylog", listener: (line: Buffer) => void): this;
    prependOnceListener(
      event: string,
      listener: (...args: any[]) => void
    ): this;
    prependOnceListener(
      event: "close",
      listener: (hadError: boolean) => void
    ): this;
    prependOnceListener(event: "secureConnect", listener: () => void): this;
    prependOnceListener(event: "data", listener: (data: Buffer) => void): this;
    prependOnceListener(event: "end", listener: () => void): this;
    prependOnceListener(event: "error", listener: (err: Error) => void): this;
    prependOnceListener(
      event: "keylog",
      listener: (line: Buffer) => void
    ): this;
  }

  /**
   * A SecureContext object holds credentials for TLS connections.
   * It can be reused across multiple TLS connections.
   */
  class SecureContext {
    constructor();
  }

  /**
   * Creates a new `tls.TLSSocket` and initiates a connection.
   *
   * @param options Connection options
   * @param callback Callback to be called when the `'secureConnect'` event is emitted
   * @returns A new `tls.TLSSocket` instance
   */
  function connect(
    options: ConnectionOptions,
    callback?: () => void
  ): TLSSocket;

  /**
   * Creates a new secure context.
   *
   * @param options Secure context options
   * @returns A new `tls.SecureContext` instance
   */
  function createSecureContext(options?: SecureContextOptions): SecureContext;

  /**
   * Returns an array with the names of the supported TLS ciphers.
   * The names are lower-case for historical reasons, but must be uppercased to
   * be used in the `ciphers` option of `tls.createSecureContext()`.
   *
   * @returns An array of cipher names in OpenSSL format
   */
  function getCiphers(): string[];

  /**
   * The default minimum TLS version used for TLS connections.
   * Currently 'TLSv1.2'.
   */
  const DEFAULT_MIN_VERSION: string;

  /**
   * The default maximum TLS version used for TLS connections.
   * Currently 'TLSv1.3'.
   */
  const DEFAULT_MAX_VERSION: string;

  /**
   * An immutable array of strings representing the root certificates (in PEM format)
   * from the bundled Mozilla CA store as supplied by the current Node.js version.
   */
  const rootCertificates: ReadonlyArray<string>;

  /**
   * Verifies the certificate `cert` is issued to `hostname`.
   *
   * Returns `undefined` if the hostname matches, or an `Error` object if it doesn't.
   * This function can be used to build a custom certificate verification function.
   *
   * @param hostname The hostname to verify against
   * @param cert The certificate object from getPeerCertificate()
   * @returns undefined on success, Error on failure
   */
  function checkServerIdentity(
    hostname: string,
    cert: PeerCertificate
  ): Error | undefined;
}
