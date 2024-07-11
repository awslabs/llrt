# API documentation

> [!NOTE]
> The long term goal for LLRT is to become [Winter CG compliant](https://github.com/wintercg/admin/blob/main/proposals.md). Not every API from Node.js will be supported.

## buffer

[alloc](https://nodejs.org/api/buffer.html#static-method-bufferallocsize-fill-encoding)

[byteLength](https://nodejs.org/api/buffer.html#static-method-bufferbytelengthstring-encoding)

[concat](https://nodejs.org/api/buffer.html#static-method-bufferconcatlist-totallength)

[from](https://nodejs.org/api/buffer.html#static-method-bufferfromarray)

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

## @llrt/hex

```typescript
export function encode(
  value: string | Array | ArrayBuffer | Uint8Array
): string;
export function decode(value: string): Uint8Array;
```

## module

[createRequire](https://nodejs.org/api/module.html#modulecreaterequirefilename)

> [!NOTE]
> `require` is available from esm modules natively. This function is just for compatibility

## os

[platform](https://nodejs.org/api/os.html#osplatform)

[release](https://nodejs.org/api/os.html#osrelease)

[tmpdir](https://nodejs.org/api/os.html#osplatform)

[type](https://nodejs.org/api/os.html#ostype)

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

### TODO, URLSearchParams see tracking [ticket](https://github.com/awslabs/llrt/issues/307):

```typescript
URLSearchParams.sort(): void;
```

## util

> [!IMPORTANT]
> Supported encodings: hex, base64, utf8, iso88591.
> Supported methods: `encode` & `decode`

[TextDecoder](https://nodejs.org/api/util.html#class-utiltextdecoder)

[TextEncoder](https://nodejs.org/api/util.html#class-utiltextdecoder)

## uuid

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

## xml

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

## net

> [!WARNING]
> These APIs uses native streams that is not 100% compatible with the Node.js Streams API. Server APIs like `createSever` provides limited functionality useful for testing purposes. Serverless applications typically don't expose servers. Some server options are not supported:
> `highWaterMark`, `pauseOnConnect`, `keepAlive`, `noDelay`, `keepAliveInitialDelay`

[connect](https://nodejs.org/api/net.html#netconnect)

[createConnection](https://nodejs.org/api/net.html#netcreateconnection)

[createServer](https://nodejs.org/api/net.html#netcreateserveroptions-connectionlistener)

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
