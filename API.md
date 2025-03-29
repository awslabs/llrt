# API documentation

> [!NOTE]
> The long term goal for LLRT is to become [Winter CG compliant](https://github.com/wintercg/admin/blob/main/proposals.md). Not every API from Node.js will be supported.

## assert

[ok](https://nodejs.org/api/assert.html#assertokvalue-message)

## buffer

### static methods

[alloc](https://nodejs.org/api/buffer.html#static-method-bufferallocsize-fill-encoding)

[allocUnsafe](https://nodejs.org/api/buffer.html#static-method-bufferallocunsafesize)

[allocUnsafeSlow](https://nodejs.org/api/buffer.html#static-method-bufferallocunsafeslowsize)

[byteLength](https://nodejs.org/api/buffer.html#static-method-bufferbytelengthstring-encoding)

[concat](https://nodejs.org/api/buffer.html#static-method-bufferconcatlist-totallength)

[from](https://nodejs.org/api/buffer.html#static-method-bufferfromarray)

[isBuffer](https://nodejs.org/api/buffer.html#static-method-bufferisbufferobj)

[isEncoding](https://nodejs.org/api/buffer.html#static-method-bufferisencodingencoding)

### prototype methods

[copy](https://nodejs.org/api/buffer.html#bufcopytarget-targetstart-sourcestart-sourceend)

[subarray](https://nodejs.org/api/buffer.html#bufsubarraystart-end)

[toString](https://nodejs.org/api/buffer.html#buftostringencoding-start-end)

[write](https://nodejs.org/api/buffer.html#bufwritestring-offset-length-encoding)

[writeBigInt64BE](https://nodejs.org/api/buffer.html#bufwritebigint64bevalue-offset)

[writeBigInt64LE](https://nodejs.org/api/buffer.html#bufwritebigint64levalue-offset)

[writeDoubleBE](https://nodejs.org/api/buffer.html#bufwritedoublebevalue-offset)

[writeDoubleLE](https://nodejs.org/api/buffer.html#bufwritedoublelevalue-offset)

[writeFloatBE](https://nodejs.org/api/buffer.html#bufwritefloatbevalue-offset)

[writeFloatLE](https://nodejs.org/api/buffer.html#bufwritefloatlevalue-offset)

[writeInt8](https://nodejs.org/api/buffer.html#bufwriteint8value-offset)

[writeInt16BE](https://nodejs.org/api/buffer.html#bufwriteint16bevalue-offset)

[writeInt16LE](https://nodejs.org/api/buffer.html#bufwriteint16levalue-offset)

[writeInt32BE](https://nodejs.org/api/buffer.html#bufwriteint32bevalue-offset)

[writeInt32LE](https://nodejs.org/api/buffer.html#bufwriteint32levalue-offset)

[writeUInt8](https://nodejs.org/api/buffer.html#bufwriteuint8value-offset)

[writeUInt16BE](https://nodejs.org/api/buffer.html#bufwriteuint16bevalue-offset)

[writeUInt16LE](https://nodejs.org/api/buffer.html#bufwriteuint16levalue-offset)

[writeUInt32BE](https://nodejs.org/api/buffer.html#bufwriteuint32bevalue-offset)

[writeUInt32LE](https://nodejs.org/api/buffer.html#bufwriteuint32levalue-offset)

### constants

[constants.MAX_LENGTH](https://nodejs.org/api/buffer.html#bufferconstantsmax_length)

[constants.MAX_STRING_LENGTH](https://nodejs.org/api/buffer.html#bufferconstantsmax_string_length)

Everything else inherited from [Uint8Array](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array)

## child_process

> [!WARNING]
> `spawn` uses native streams that is not 100% compatible with the Node.js Streams API.

[spawn](https://nodejs.org/api/child_process.html#child_processspawncommand-args-options)

## console

[Console](https://nodejs.org/api/console.html#class-console)

## crypto

[createHash](https://nodejs.org/api/crypto.html#cryptocreatehashalgorithm-options)

[createHmac](https://nodejs.org/api/crypto.html#cryptocreatehmacalgorithm-key-options)

[getRandomValues](https://nodejs.org/api/crypto.html#cryptogetrandomvaluestypedarray)

[randomBytes](https://nodejs.org/api/crypto.html#cryptorandombytessize-callback)

[randomFill](https://nodejs.org/api/crypto.html#cryptorandomfillbuffer-offset-size-callback)

[randomFillSync](https://nodejs.org/api/crypto.html#cryptorandomfillsyncbuffer-offset-size)

[randomInt](https://nodejs.org/api/crypto.html#cryptorandomintmin-max-callback)

[randomUUID](https://nodejs.org/api/crypto.html#cryptorandomuuidoptions)

## crypto.subtle

[subtle.decrypt](https://nodejs.org/api/webcrypto.html#subtledecryptalgorithm-key-data)

[subtle.deriveBits](https://nodejs.org/api/webcrypto.html#subtlederivebitsalgorithm-basekey-length)

[subtle.digest](https://nodejs.org/api/webcrypto.html#subtledigestalgorithm-data)

[subtle.encrypt](https://nodejs.org/api/webcrypto.html#subtleencryptalgorithm-key-data)

[subtle.exportKey](https://nodejs.org/api/webcrypto.html#subtleexportkeyformat-key)

[subtle.generateKey](https://nodejs.org/api/webcrypto.html#subtlegeneratekeyalgorithm-extractable-keyusages)

[subtle.importKey](https://nodejs.org/api/webcrypto.html#subtleimportkeyformat-keydata-algorithm-extractable-keyusages)

[subtle.sign](https://nodejs.org/api/webcrypto.html#subtlesignalgorithm-key-data)

[subtle.verify](https://nodejs.org/api/webcrypto.html#subtleverifyalgorithm-key-signature-datah)

## dns

[lookup](https://nodejs.org/api/dns.html#dnslookuphostname-options-callback)

## events

[EventEmitter](https://nodejs.org/api/events.html#class-eventemitter)

## fetch

Available globally

[fetch](https://developer.mozilla.org/en-US/docs/Web/API/fetch)

> [!IMPORTANT]
> There are some differences with the [WHATWG standard](https://fetch.spec.whatwg.org). Mainly browser specific behavior is removed:
>
> - `keepalive` is always true
> - `request.body` can only be `string`, `Array`, `ArrayBuffer` or `Uint8Array`
> - `response.body` returns `null`. Use `response.text()`, `response.json()` etc
> - `mode`, `credentials`, `referrerPolicy`, `priority`, `cache` is not available/applicable

## string_decoder

[StringDecoder](https://nodejs.org/api/string_decoder.html#class-stringdecoder)

## file

[file](https://developer.mozilla.org/en-US/docs/Web/API/File)

## fs

[accessSync](https://nodejs.org/api/fs.html#fsaccesssyncpath-mode)

[mkdirSync](https://nodejs.org/api/fs.html#fsmkdirsyncpath-options)

[mkdtempSync](https://nodejs.org/api/fs.html#fsmkdtempsyncprefix-options)

[readdirSync](https://nodejs.org/api/fs.html#fsreaddirsyncpath-options)

[readFileSync](https://nodejs.org/api/fs.html#fsreadfilesyncpath-options)

[rmdirSync](https://nodejs.org/api/fs.html#fsrmdirsyncpath-options)

[rmSync](https://nodejs.org/api/fs.html#fsrmsyncpath-options)

[statSync](https://nodejs.org/api/fs.html#fsstatsyncpath-options)

[writeFileSync](https://nodejs.org/api/fs.html#fswritefilesyncfile-data-options)

[chmodSync](https://nodejs.org/api/fs.html#fschmodsyncpath-mode)

## fs/promises

[access](https://nodejs.org/api/fs.html#fsstatpath-options-callback)

[constants](https://nodejs.org/api/fs.html#file-access-constants)

[mkdir](https://nodejs.org/api/fs.html#fsmkdirpath-options-callback)

[mkdtemp](https://nodejs.org/api/fs.html#fsmkdtempprefix-options-callback)

[readdir](https://nodejs.org/api/fs.html#fspromisesreaddirpath-options)

[readFile](https://nodejs.org/api/fs.html#filehandlereadfileoptions)

[rm](https://nodejs.org/api/fs.html#fsrmpath-options-callback)

[rmdir](https://nodejs.org/api/fs.html#fsrmdirpath-options-callback)

[stat](https://nodejs.org/api/fs.html#fsstatpath-options-callback)

[writeFile](https://nodejs.org/api/fs.html#fspromiseswritefilefile-data-options)

[chmod](https://nodejs.org/api/fs.html#fspromiseschmodpath-mode)

## module

[createRequire](https://nodejs.org/api/module.html#modulecreaterequirefilename)

> [!NOTE]
> `require` is available from esm modules natively. This function is just for compatibility

## net

> [!WARNING]
> These APIs uses native streams that is not 100% compatible with the Node.js Streams API. Server APIs like `createSever` provides limited functionality useful for testing purposes. Serverless applications typically don't expose servers. Some server options are not supported:
> `highWaterMark`, `pauseOnConnect`, `keepAlive`, `noDelay`, `keepAliveInitialDelay`

[connect](https://nodejs.org/api/net.html#netconnect)

[createConnection](https://nodejs.org/api/net.html#netcreateconnection)

[createServer](https://nodejs.org/api/net.html#netcreateserveroptions-connectionlistener)

## os

[arch](https://nodejs.org/api/os.html#osarch)

[availableParallelism](https://nodejs.org/api/os.html#osavailableparallelism)

[cpus](https://nodejs.org/api/os.html#oscpus)

[devNull](https://nodejs.org/api/os.html#osdevnull)

[endianness](https://nodejs.org/api/os.html#osendianness)

[EOL](https://nodejs.org/api/os.html#oseol)

[freemem](https://nodejs.org/api/os.html#osfreemem)

[getPriority](https://nodejs.org/api/os.html#osgetprioritypid)

[homedir](https://nodejs.org/api/os.html#oshomedir)

[hostname](https://nodejs.org/api/os.html#oshostname)

[loadavg](https://nodejs.org/api/os.html#osloadavg)

[machine](https://nodejs.org/api/os.html#osmachine)

[networkInterfaces](https://nodejs.org/api/os.html#osnetworkinterfaces)

[platform](https://nodejs.org/api/os.html#osplatform)

[release](https://nodejs.org/api/os.html#osrelease)

[setPriority](https://nodejs.org/api/os.html#ossetprioritypid-priority)

[tmpdir](https://nodejs.org/api/os.html#osplatform)

[totalmem](https://nodejs.org/api/os.html#ostotalmem)

[type](https://nodejs.org/api/os.html#ostype)

[uptime](https://nodejs.org/api/os.html#osuptime)

[userInfo](https://nodejs.org/api/os.html#osuserinfooptions)

[version](https://nodejs.org/api/os.html#osversion)

## path

[basename](https://nodejs.org/api/path.html#pathbasenamepath-suffix)

[delimiter](https://nodejs.org/api/path.html#pathdelimiter)

[dirname](https://nodejs.org/api/path.html#pathdirnamepath)

[extname](https://nodejs.org/api/path.html#pathextnamepath)

[format](https://nodejs.org/api/path.html#pathformatpathobject)

[isAbsolute](https://nodejs.org/api/path.html#pathisabsolutepath)

[join](https://nodejs.org/api/path.html#pathjoinpaths)

[normalize](https://nodejs.org/api/path.html#pathnormalizepath)

[parse](https://nodejs.org/api/path.html#pathparsepath)

[resolve](https://nodejs.org/api/path.html#pathresolvepaths)

## timers

_Also available globally_

[clearImmediate](https://nodejs.org/api/timers.html#clearimmediateimmediate)

[clearInterval](https://nodejs.org/api/timers.html#clearintervaltimeout)

[clearTimeout](https://nodejs.org/api/timers.html#cleartimeouttimeout)

[setImmediate](https://nodejs.org/api/timers.html#setimmediatecallback-args)

[setInterval](https://nodejs.org/api/timers.html#setintervalcallback-delay-args)

[setTimeout](https://nodejs.org/api/timers.html#settimeoutcallback-delay-args)

## url

```typescript
export class URL {
  constructor(input: string, base?: string | URL);

  hash: string;
  host: string;
  hostname: string;
  href: string;
  origin: string;
  password: string;
  pathname: string;
  port: string;
  protocol: string;
  search: string;
  searchParams: URLSearchParams;
  username: string;

  parse(input: string, base?: string): URL | null;
  canParse(input: string, base?: string): boolean;
  toJSON(): string;
  toString(): string;
}
```

```typescript
// Additional utilities in the URL module
export function domainToASCII(domain: string): string;

export function domainToUnicode(domain: string): string;

export function fileURLToPath(url: string | URL): string;

export function pathToFileURL(path: string): URL;

export function format(url: string | URL, options?: { fragment?: boolean, unicode?: boolean, auth?: boolean
}): string;

export function urlToHttpOptions(url: URL): {
  protocol?: string;
  hostname?: string;
  port?: string;
  path?: string;
  ...
};
```

## URLSearchParams

```typescript
export class URLSearchParams {
  constructor(
    init?: string | string[][] | Record<string, string> | URLSearchParams
  );

  // properties
  size: number;

  // Methods
  append(name: string, value: string): void;
  delete(name: string): void;
  get(name: string): string | null;
  getAll(name: string): string[];
  has(name: string): boolean;
  set(name: string, value: string): void;
  sort(): void;

  [Symbol.iterator](): IterableIterator<[string, string]>;
  entries(): IterableIterator<[string, string]>;
  forEach(): IterableIterator<[string, string]>;
  keys(): IterableIterator<string>;
  values(): IterableIterator<string>;

  toString(): string;
}
```

## util

> [!IMPORTANT]
> Supported encodings: hex, base64, utf-8, utf-16le, windows-1252 and their aliases.

[format](https://nodejs.org/api/util.html#utilformatformat-args)

[inherits](https://nodejs.org/api/util.html#utilinheritsconstructor-superconstructor)

[TextDecoder](https://nodejs.org/api/util.html#class-utiltextdecoder)

[TextEncoder](https://nodejs.org/api/util.html#class-utiltextdecoder)

## zlib

[deflate](https://nodejs.org/api/zlib.html#zlibdeflatebuffer-options-callback)

[deflateSync](https://nodejs.org/api/zlib.html#zlibdeflatesyncbuffer-options)

[deflateRaw](https://nodejs.org/api/zlib.html#zlibdeflaterawbuffer-options-callback)

[deflateRawSync](https://nodejs.org/api/zlib.html#zlibdeflaterawsyncbuffer-options)

[gzip](https://nodejs.org/api/zlib.html#zlibgzipbuffer-options-callback)

[gzipSync](https://nodejs.org/api/zlib.html#zlibgzipsyncbuffer-options)

[inflate](https://nodejs.org/api/zlib.html#zlibinflatebuffer-options-callback)

[inflateSync](https://nodejs.org/api/zlib.html#zlibinflatesyncbuffer-options)

[inflateRaw](https://nodejs.org/api/zlib.html#zlibinflaterawbuffer-options-callback)

[inflateRawSync](https://nodejs.org/api/zlib.html#zlibinflaterawsyncbuffer-options)

[gunzip](https://nodejs.org/api/zlib.html#zlibgunzipbuffer-options-callback)

[gunzipSync](https://nodejs.org/api/zlib.html#zlibgunzipsyncbuffer-options)

[brotliCompress](https://nodejs.org/api/zlib.html#zlibbrotlicompressbuffer-options-callback)

[brotliCompressSync](https://nodejs.org/api/zlib.html#zlibbrotlicompresssyncbuffer-options)

[brotliDecompress](https://nodejs.org/api/zlib.html#zlibbrotlidecompressbuffer-options-callback)

[brotliDecompressSync](https://nodejs.org/api/zlib.html#zlibbrotlidecompresssyncbuffer-options)

## llrt:hex

```typescript
export function encode(
  value: string | Array | ArrayBuffer | Uint8Array
): string;
export function decode(value: string): Uint8Array;
```

## llrt:uuid

```typescript
export const NIL: string;

export function v1(): string;

export function v3(
  name: string,
  namespace: Array | Uint8Array | String
): string;

export function v4(): string;

export function v5(
  name: string,
  namespace: Array | Uint8Array | String
): string;

export function parse(value: string): Uint8Array;

export function stringify(arr: Array | Uint8Array): string;

export function validate(arr: string): boolean;

export function version(arr: Array | Uint8Array): number;
```

## llrt:xml

A lightweight and fast XML parser

```typescript
type XmlParserOptions = {
    ignoreAttributes?: boolean;
    attributeNamePrefix?: string;
    textNodeName?: string;
    attributeValueProcessor?: (attrName: string, attrValue: string, jpath: string) => unknown;
    tagValueProcessor?: (attrName: string, attrValue: string, jpath: string, hasAttributes: boolean) => unknown;
}
export class XMLParser(options?: XmlParserOptions){
    parse(xml:string):object
}

```

## Misc Global objects

[AbortController](https://developer.mozilla.org/en-US/docs/Web/API/AbortController)

[AbortSignal](https://developer.mozilla.org/en-US/docs/Web/API/AbortSignal)

[atob](https://developer.mozilla.org/en-US/docs/Web/API/atob)

[btoa](https://developer.mozilla.org/en-US/docs/Web/API/btoa)

[DOMException](https://developer.mozilla.org/en-US/docs/Web/API/DOMException)

[navigator.userAgent](https://nodejs.org/api/globals.html#navigatoruseragent)

[performance.now](https://nodejs.org/api/perf_hooks.html#performancenow)

[performance.timeOrigin](https://nodejs.org/api/perf_hooks.html#performancetimeorigin)

[structuredClone](https://nodejs.org/api/globals.html#structuredclonevalue-options)
