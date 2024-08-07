/**
 * The `net` module provides an asynchronous network API for creating stream-based
 * TCP or `IPC` servers ({@link createServer}) and clients ({@link createConnection}).
 *
 * It can be accessed using:
 *
 * ```js
 * import * as net from 'net';
 * ```
 */
declare module "net" {
  import { Buffer } from "buffer";
  import { EventEmitter } from "events";
  import { DefaultDuplexStream as Duplex } from "stream";

  interface AddressInfo {
    address: string;
    family: string;
    port: number;
  }

  interface SocketConstructorOpts {
    allowHalfOpen?: boolean | undefined;
  }
  interface TcpSocketConnectOpts {
    port: number;
    host?: string | undefined;
  }
  interface IpcSocketConnectOpts {
    path: string;
  }
  type SocketConnectOpts = TcpSocketConnectOpts | IpcSocketConnectOpts;
  type SocketReadyState =
    | "opening"
    | "open"
    | "readOnly"
    | "writeOnly"
    | "closed";

  /**
   * This class is an abstraction of a TCP socket or a streaming `IPC` endpoint (uses named pipes on Windows, and Unix domain sockets otherwise).
   * It is also an `EventEmitter`.
   *
   * A `net.Socket` can be created by the user and used directly to interact with a server. For example, it is returned by {@link createConnection},
   * so the user can use it to talk to the server.
   *
   * It can also be created by LLRT and passed to the user when a connection is received.
   * For example, it is passed to the listeners of a `'connection'` event emitted on a {@link Server}, so the user can use it to interact with the client.
   */
  class Socket extends Duplex {
    constructor(options?: SocketConstructorOpts);

    /**
     * Initiate a connection on a given socket.
     *
     * Possible signatures:
     *
     * * `socket.connect(options[, connectListener])`
     * * `socket.connect(path[, connectListener])` for `IPC` connections.
     * * `socket.connect(port[, host][, connectListener])` for TCP connections.
     * * Returns: `net.Socket` The socket itself.
     *
     * This function is asynchronous. When the connection is established, the `'connect'` event will be emitted. If there is a problem connecting,
     * instead of a `'connect'` event, an `'error'` event will be emitted with
     * the error passed to the `'error'` listener.
     * The last parameter `connectListener`, if supplied, will be added as a listener
     * for the `'connect'` event **once**.
     *
     * This function should only be used for reconnecting a socket after`'close'` has been emitted or otherwise it may lead to undefined
     * behavior.
     */
    connect(options: SocketConnectOpts, connectionListener?: () => void): this;
    connect(port: number, host: string, connectionListener?: () => void): this;
    connect(port: number, connectionListener?: () => void): this;
    connect(path: string, connectionListener?: () => void): this;

    /**
     * Returns the bound `address`, the address `family` name and `port` of the
     * socket as reported by the operating system:`{ port: 12346, family: 'IPv4', address: '127.0.0.1' }`
     * @since v0.1.90
     */
    address(): AddressInfo | {};

    /**
     * If `true`, `socket.connect(options[, connectListener])` was
     * called and has not yet finished. It will stay `true` until the socket becomes
     * connected, then it is set to `false` and the `'connect'` event is emitted. Note
     * that the `socket.connect(options[, connectListener])` callback is a listener for the `'connect'` event.
     */
    readonly connecting: boolean;

    /**
     * This is `true` if the socket is not connected yet, either because `.connect()`has not yet been called or because it is still in the process of connecting
     * (see `socket.connecting`).
     */
    readonly pending: boolean;

    /**
     * The string representation of the local IP address the remote client is
     * connecting on. For example, in a server listening on `'0.0.0.0'`, if a client
     * connects on `'192.168.1.1'`, the value of `socket.localAddress` would be`'192.168.1.1'`.
     */
    readonly localAddress?: string;

    /**
     * The numeric representation of the local port. For example, `80` or `21`.
     */
    readonly localPort?: number;

    /**
     * The string representation of the local IP family. `'IPv4'` or `'IPv6'`.
     */
    readonly localFamily?: string;

    /**
     * This property represents the state of the connection as a string.
     *
     * * If the stream is connecting `socket.readyState` is `opening`.
     * * If the stream is readable and writable, it is `open`.
     * * If the stream is readable and not writable, it is `readOnly`.
     * * If the stream is not readable and writable, it is `writeOnly`.
     */
    readonly readyState: SocketReadyState;

    /**
     * The string representation of the remote IP address. For example,`'74.125.127.100'` or `'2001:4860:a005::68'`. Value may be `undefined` if
     * the socket is destroyed (for example, if the client disconnected).
     */
    readonly remoteAddress?: string;

    /**
     * The string representation of the remote IP family. `'IPv4'` or `'IPv6'`. Value may be `undefined` if
     * the socket is destroyed (for example, if the client disconnected).
     */
    readonly remoteFamily?: string;

    /**
     * The numeric representation of the remote port. For example, `80` or `21`. Value may be `undefined` if
     * the socket is destroyed (for example, if the client disconnected).
     */
    readonly remotePort?: number | undefined;

    /**
     * Half-closes the socket. i.e., it sends a FIN packet. It is possible the server will still send some data.
     *
     * @param callback Optional callback for when the socket is finished.
     * @return The socket itself.
     */
    end(callback?: () => void): this;

    /**
     * events.EventEmitter
     *   1. close
     *   2. connect
     *   3. data
     *   4. end
     *   5. error
     */
    addListener(event: string, listener: (...args: any[]) => void): this;
    addListener(event: "close", listener: (hadError: boolean) => void): this;
    addListener(event: "connect", listener: () => void): this;
    addListener(event: "data", listener: (data: Buffer) => void): this;
    addListener(event: "drain", listener: () => void): this;
    addListener(event: "end", listener: () => void): this;
    addListener(event: "error", listener: (err: Error) => void): this;
    emit(event: string | symbol, ...args: any[]): boolean;
    emit(event: "close", hadError: boolean): boolean;
    emit(event: "connect"): boolean;
    emit(event: "data", data: Buffer): boolean;
    emit(event: "end"): boolean;
    emit(event: "error", err: Error): boolean;
    on(event: string, listener: (...args: any[]) => void): this;
    on(event: "close", listener: (hadError: boolean) => void): this;
    on(event: "connect", listener: () => void): this;
    on(event: "data", listener: (data: Buffer) => void): this;
    on(event: "end", listener: () => void): this;
    on(event: "error", listener: (err: Error) => void): this;
    once(event: string, listener: (...args: any[]) => void): this;
    once(event: "close", listener: (hadError: boolean) => void): this;
    once(event: "connect", listener: () => void): this;
    once(event: "data", listener: (data: Buffer) => void): this;
    once(event: "end", listener: () => void): this;
    once(event: "error", listener: (err: Error) => void): this;
    prependListener(event: string, listener: (...args: any[]) => void): this;
    prependListener(
      event: "close",
      listener: (hadError: boolean) => void
    ): this;
    prependListener(event: "connect", listener: () => void): this;
    prependListener(event: "data", listener: (data: Buffer) => void): this;
    prependListener(event: "end", listener: () => void): this;
    prependListener(event: "error", listener: (err: Error) => void): this;
    prependOnceListener(
      event: string,
      listener: (...args: any[]) => void
    ): this;
    prependOnceListener(
      event: "close",
      listener: (hadError: boolean) => void
    ): this;
    prependOnceListener(event: "connect", listener: () => void): this;
    prependOnceListener(event: "data", listener: (data: Buffer) => void): this;
    prependOnceListener(event: "end", listener: () => void): this;
    prependOnceListener(event: "error", listener: (err: Error) => void): this;
  }

  interface ListenOptions {
    port?: number | undefined;
    host?: string | undefined;
    backlog?: number | undefined;
    path?: string | undefined;
  }

  interface ServerOpts {
    /**
     * Indicates whether half-opened TCP connections are allowed.
     * @default false
     */
    allowHalfOpen?: boolean | undefined;
  }

  /**
   * This class is used to create a TCP or `IPC` server.
   */
  class Server extends EventEmitter {
    constructor(connectionListener?: (socket: Socket) => void);
    constructor(
      options?: ServerOpts,
      connectionListener?: (socket: Socket) => void
    );

    /**
     * Start a server listening for connections. A `net.Server` can be a TCP or
     * an `IPC` server depending on what it listens to.
     *
     * Possible signatures:
     *
     * * `server.listen(options[, callback])`
     * * `server.listen(path[, backlog][, callback])` for `IPC` servers
     * * `server.listen([port[, host[, backlog]]][, callback])` for TCP servers
     *
     * This function is asynchronous. When the server starts listening, the `'listening'` event will be emitted. The last parameter `callback`will be added as a listener for the `'listening'`
     * event.
     *
     * All `listen()` methods can take a `backlog` parameter to specify the maximum length of the queue of pending connections.
     * Currently this parameter is IGNORED, support will be added in the future.
     *
     * All {@link Socket} are set to `SO_REUSEADDR` (see [`socket(7)`](https://man7.org/linux/man-pages/man7/socket.7.html) for details).
     *
     * The `server.listen()` method can be called again if and only if there was an error during the first `server.listen()`
     * call or `server.close()` has been called. Otherwise, an error will be thrown.
     */
    listen(listeningListener?: () => void): void;
    listen(
      port?: number,
      hostname?: string,
      backlog?: number,
      listeningListener?: () => void
    ): void;
    listen(
      port?: number,
      hostname?: string,
      listeningListener?: () => void
    ): void;
    listen(
      port?: number,
      backlog?: number,
      listeningListener?: () => void
    ): void;
    listen(port?: number, listeningListener?: () => void): void;
    listen(
      path: string,
      backlog?: number,
      listeningListener?: () => void
    ): void;
    listen(path: string, listeningListener?: () => void): void;
    listen(options: ListenOptions, listeningListener?: () => void): void;

    /**
     * Stops the server from accepting new connections and keeps existing
     * connections. This function is asynchronous, the server is finally closed
     * when all connections are ended and the server emits a `'close'` event.
     * The optional `callback` will be called once the `'close'` event occurs. Unlike
     * that event, it will be called with an `Error` as its only argument if the server
     * was not open when it was closed.
     * @param callback Called when the server is closed.
     */
    close(callback?: (err?: Error) => void): this;

    /**
     * Returns the bound `address`, the address `family` name, and `port` of the server
     * as reported by the operating system if listening on an IP socket
     * (useful to find which port was assigned when getting an OS-assigned address):`{ port: 12346, family: 'IPv4', address: '127.0.0.1' }`.
     *
     * For a server listening on a pipe or Unix domain socket, the name is returned
     * as a string.
     *
     * ```js
     * const server = net.createServer((socket) => {
     *   socket.end('goodbye\n');
     * }).on('error', (err) => {
     *   // Handle errors here.
     *   throw err;
     * });
     *
     * // Grab an arbitrary unused port.
     * server.listen(() => {
     *   console.log('opened server on', server.address());
     * });
     * ```
     *
     * `server.address()` returns `null` before the `'listening'` event has been
     * emitted or after calling `server.close()`.
     */
    address(): AddressInfo | string | null;

    /**
     * events.EventEmitter
     *   1. close
     *   2. connection
     *   3. error
     *   4. listening
     */
    addListener(event: string, listener: (...args: any[]) => void): this;
    addListener(event: "close", listener: () => void): this;
    addListener(event: "connection", listener: (socket: Socket) => void): this;
    addListener(event: "error", listener: (err: Error) => void): this;
    addListener(event: "listening", listener: () => void): this;
    emit(event: string | symbol, ...args: any[]): boolean;
    emit(event: "close"): boolean;
    emit(event: "connection", socket: Socket): boolean;
    emit(event: "error", err: Error): boolean;
    emit(event: "listening"): boolean;
    on(event: string, listener: (...args: any[]) => void): this;
    on(event: "close", listener: () => void): this;
    on(event: "connection", listener: (socket: Socket) => void): this;
    on(event: "error", listener: (err: Error) => void): this;
    on(event: "listening", listener: () => void): this;
    once(event: string, listener: (...args: any[]) => void): this;
    once(event: "close", listener: () => void): this;
    once(event: "connection", listener: (socket: Socket) => void): this;
    once(event: "error", listener: (err: Error) => void): this;
    once(event: "listening", listener: () => void): this;
    prependListener(event: string, listener: (...args: any[]) => void): this;
    prependListener(event: "close", listener: () => void): this;
    prependListener(
      event: "connection",
      listener: (socket: Socket) => void
    ): this;
    prependListener(event: "error", listener: (err: Error) => void): this;
    prependListener(event: "listening", listener: () => void): this;
    prependOnceListener(
      event: string,
      listener: (...args: any[]) => void
    ): this;
    prependOnceListener(event: "close", listener: () => void): this;
    prependOnceListener(
      event: "connection",
      listener: (socket: Socket) => void
    ): this;
    prependOnceListener(event: "error", listener: (err: Error) => void): this;
    prependOnceListener(event: "listening", listener: () => void): this;
  }

  type NetConnectOpts = TcpSocketConnectOpts | IpcSocketConnectOpts;

  /**
   * Creates a new TCP or `IPC` server.
   *
   * If `allowHalfOpen` is set to `true`, when the other end of the socket
   * signals the end of transmission, the server will only send back the end of
   * transmission when `socket.end()` is explicitly called. For example, in the
   * context of TCP, when a FIN packed is received, a FIN packed is sent
   * back only when `socket.end()` is explicitly called. Until then the
   * connection is half-closed (non-readable but still writable). See `'end'` event and [RFC 1122](https://tools.ietf.org/html/rfc1122) (section 4.2.2.13) for more information.
   *
   * If `pauseOnConnect` is set to `true`, then the socket associated with each
   * incoming connection will be paused, and no data will be read from its handle.
   * This allows connections to be passed between processes without any data being
   * read by the original process. To begin reading data from a paused socket, call `socket.resume()`.
   *
   * The server can be a TCP server or an `IPC` server, depending on what it `listen()` to.
   *
   * Here is an example of a TCP echo server which listens for connections
   * on port 8124:
   *
   * ```js
   * import * as net from 'net';
   * const server = net.createServer((c) => {
   *   // 'connection' listener.
   *   console.log('client connected');
   *   c.on('end', () => {
   *     console.log('client disconnected');
   *   });
   *   c.write('hello\r\n');
   *
   * });
   * server.on('error', (err) => {
   *   throw err;
   * });
   * server.listen(8124, () => {
   *   console.log('server bound');
   * });
   * ```
   *
   * Test this by using `telnet`:
   *
   * ```bash
   * telnet localhost 8124
   * ```
   *
   * To listen on the socket `/tmp/echo.sock`:
   *
   * ```js
   * server.listen('/tmp/echo.sock', () => {
   *   console.log('server bound');
   * });
   * ```
   *
   * Use `nc` to connect to a Unix domain socket server:
   *
   * ```bash
   * nc -U /tmp/echo.sock
   * ```
   * @param connectionListener Automatically set as a listener for the {@link 'connection'} event.
   */
  function createServer(connectionListener?: (socket: Socket) => void): Server;
  function createServer(
    options?: ServerOpts,
    connectionListener?: (socket: Socket) => void
  ): Server;

  /**
   * Aliases to {@link createConnection}.
   *
   * Possible signatures:
   *
   * * {@link connect}
   * * {@link connect} for `IPC` connections.
   * * {@link connect} for TCP connections.
   */
  function connect(
    options: NetConnectOpts,
    connectionListener?: () => void
  ): Socket;
  function connect(
    port: number,
    host: string,
    connectionListener?: () => void
  ): Socket;
  function connect(port: number, connectionListener?: () => void): Socket;
  function connect(path: string, connectionListener?: () => void): Socket;

  /**
   * A factory function, which creates a new {@link Socket},
   * immediately initiates connection with `socket.connect()`,
   * then returns the `net.Socket` that starts the connection.
   *
   * When the connection is established, a `'connect'` event will be emitted
   * on the returned socket. The last parameter `connectListener`, if supplied,
   * will be added as a listener for the `'connect'` event **once**.
   *
   * Possible signatures:
   *
   * * {@link createConnection}
   * * {@link createConnection} for `IPC` connections.
   * * {@link createConnection} for TCP connections.
   *
   * The {@link connect} function is an alias to this function.
   */
  function createConnection(
    options: NetConnectOpts,
    connectionListener?: () => void
  ): Socket;
  function createConnection(
    port: number,
    host: string,
    connectionListener?: () => void
  ): Socket;
  function createConnection(
    port: number,
    connectionListener?: () => void
  ): Socket;
  function createConnection(
    path: string,
    connectionListener?: () => void
  ): Socket;
}
