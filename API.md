# API documentation

> [!NOTE]
> The long term goal for LLRT is to become [WinterTC compliant](https://min-common-api.proposal.wintertc.org/). Not every API from Node.js will be supported.

# Node.js API

## assert

[ok](https://nodejs.org/api/assert.html#assertokvalue-message)

## async_hooks

### Static methods

[createHook](https://nodejs.org/api/async_hooks.html#async_hookscreatehookcallbacks)

[executionAsyncId](https://nodejs.org/api/async_hooks.html#async_hooksexecutionasyncid)

[triggerAsyncId](https://nodejs.org/api/async_hooks.html#async_hookstriggerasyncid)

### Class: AsyncHook

[enable](https://nodejs.org/api/async_hooks.html#asynchookenable)

[disable](https://nodejs.org/api/async_hooks.html#asynchookdisable)

#### Hook callbacks

[init](https://nodejs.org/api/async_hooks.html#initasyncid-type-triggerasyncid-resource)

[before](https://nodejs.org/api/async_hooks.html#beforeasyncid)

[after](https://nodejs.org/api/async_hooks.html#afterasyncid)

[destroy](https://nodejs.org/api/async_hooks.html#destroyasyncid)

[promiseResolve](https://nodejs.org/api/async_hooks.html#promiseresolveasyncid)

## buffer

### Static methods

[alloc](https://nodejs.org/api/buffer.html#static-method-bufferallocsize-fill-encoding)

[allocUnsafe](https://nodejs.org/api/buffer.html#static-method-bufferallocunsafesize)

[allocUnsafeSlow](https://nodejs.org/api/buffer.html#static-method-bufferallocunsafeslowsize)

[byteLength](https://nodejs.org/api/buffer.html#static-method-bufferbytelengthstring-encoding)

[concat](https://nodejs.org/api/buffer.html#static-method-bufferconcatlist-totallength)

[from](https://nodejs.org/api/buffer.html#static-method-bufferfromarray)

[isBuffer](https://nodejs.org/api/buffer.html#static-method-bufferisbufferobj)

[isEncoding](https://nodejs.org/api/buffer.html#static-method-bufferisencodingencoding)

### Prototype methods

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

### Constants

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

[webcrypto](https://nodejs.org/api/crypto.html#cryptowebcrypto)

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

## dgram

[createSocket](https://nodejs.org/api/dgram.html#dgramcreatesocketoptions-callback)

### Class: dgram.Socket

[address](https://nodejs.org/api/dgram.html#socketaddress)

[bind](https://nodejs.org/api/dgram.html#socketbindport-address-callback)

[close](https://nodejs.org/api/dgram.html#socketclosecallback)

[ref](https://nodejs.org/api/dgram.html#socketref)

[send](https://nodejs.org/api/dgram.html#socketsendmsg-offset-length-port-address-callback)

[unref](hhttps://nodejs.org/api/dgram.html#socketunref)

## dns

[lookup](https://nodejs.org/api/dns.html#dnslookuphostname-options-callback)

## events

[EventEmitter](https://nodejs.org/api/events.html#class-eventemitter)

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

[renameSync](https://nodejs.org/api/fs.html#fsrenamesyncoldpath-newpath)

[symlinkSync](https://nodejs.org/api/fs.html#fssymlinksynctarget-path-type)

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

[rename](https://nodejs.org/api/fs.html#fspromisesrenameoldpath-newpath)

[symlink](https://nodejs.org/api/fs.html#fspromisessymlinktarget-path-type)

## module

[builtinModules](https://nodejs.org/api/module.html#modulebuiltinmodules)

[createRequire](https://nodejs.org/api/module.html#modulecreaterequirefilename)

> [!NOTE]
> `require` is available from esm modules natively. This function is just for compatibility

[isBuiltin](https://nodejs.org/api/module.html#moduleisbuiltinmodulename)

[registerHooks](https://nodejs.org/api/module.html#moduleregisterhooksoptions)

## net

> [!WARNING]
> These APIs uses native streams that is not 100% compatible with the Node.js Streams API. Server APIs like `createSever` provides limited functionality useful for testing purposes. Serverless applications typically don't expose servers. Some server options are not supported:
> `highWaterMark`, `pauseOnConnect`, `keepAlive`, `noDelay`, `keepAliveInitialDelay`

[connect](https://nodejs.org/api/net.html#netconnect)

[createConnection](https://nodejs.org/api/net.html#netcreateconnection)

[createServer](https://nodejs.org/api/net.html#netcreateserveroptions-connectionlistener)

## tls

> [!WARNING]
> These APIs uses native streams that is not 100% compatible with the Node.js Streams API.

[connect](https://nodejs.org/api/tls.html#tlsconnectoptions-callback)

[createSecureContext](https://nodejs.org/api/tls.html#tlscreatesecurecontextoptions)

[getCiphers](https://nodejs.org/api/tls.html#tlsgetciphers)

[rootCertificates](https://nodejs.org/api/tls.html#tlsrootcertificates)

[checkServerIdentity](https://nodejs.org/api/tls.html#tlscheckserveridentityhostname-cert)

[DEFAULT_MIN_VERSION](https://nodejs.org/api/tls.html#tlsdefault_min_version)

[DEFAULT_MAX_VERSION](https://nodejs.org/api/tls.html#tlsdefault_max_version)

[TLSSocket](https://nodejs.org/api/tls.html#class-tlstlssocket)

[SecureContext](https://nodejs.org/api/tls.html#tlscreatesecurecontextoptions)

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

## string_decoder

[StringDecoder](https://nodejs.org/api/string_decoder.html#class-stringdecoder)

## timers

_Also available globally_

[clearImmediate](https://nodejs.org/api/timers.html#clearimmediateimmediate)

[clearInterval](https://nodejs.org/api/timers.html#clearintervaltimeout)

[clearTimeout](https://nodejs.org/api/timers.html#cleartimeouttimeout)

[setImmediate](https://nodejs.org/api/timers.html#setimmediatecallback-args)

[setInterval](https://nodejs.org/api/timers.html#setintervalcallback-delay-args)

[setTimeout](https://nodejs.org/api/timers.html#settimeoutcallback-delay-args)

## url

### Class

[URL](https://nodejs.org/api/url.html#class-url)

[URLSearchParams](https://nodejs.org/api/url.html#class-urlsearchparams)

### Prototype methods

[domainToASCII](https://nodejs.org/api/url.html#urldomaintoasciidomain)

[domainToUnicode](https://nodejs.org/api/url.html#urldomaintounicodedomain)

[fileURLToPath](https://nodejs.org/api/url.html#urlfileurltopathurl-options)

[format](https://nodejs.org/api/url.html#urlformaturl-options)

[pathToFileURL](https://nodejs.org/api/url.html#urlpathtofileurlpath-options)

[urlToHttpOptions](https://nodejs.org/api/url.html#urlurltohttpoptionsurl)

## util

> [!IMPORTANT]
> Supported encodings: hex, base64, utf-8, utf-16le, windows-1252 and their aliases.

[format](https://nodejs.org/api/util.html#utilformatformat-args)

[inherits](https://nodejs.org/api/util.html#utilinheritsconstructor-superconstructor)

[TextDecoder](https://nodejs.org/api/util.html#class-utiltextdecoder)

[TextEncoder](https://nodejs.org/api/util.html#class-utiltextdecoder)

## zlib

### Convenience methods

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

[zstdCompress](https://nodejs.org/api/zlib.html#zlibzstdcompressbuffer-options-callback)

[zstdCompressSync](https://nodejs.org/api/zlib.html#zlibzstdcompresssyncbuffer-options)

[zstdDecompress](https://nodejs.org/api/zlib.html#zlibzstddecompressbuffer-options-callback)

[zstdDecompressSync](https://nodejs.org/api/zlib.html#zlibzstddecompresssyncbuffer-options)

# LLRT API

## llrt:hex

```typescript
export function encode(
  value: string | Array | ArrayBuffer | Uint8Array
): string;
export function decode(value: string): Uint8Array;
```

## llrt:timezone

Lightweight timezone support for LLRT. Provides timezone offset calculations and a minimal `Intl.DateTimeFormat` implementation for dayjs and similar library compatibility.

```typescript
interface Timezone {
  /**
   * Get the UTC offset in minutes for a timezone at a given time.
   * Returns a positive value for timezones ahead of UTC (e.g., 540 for Asia/Tokyo)
   * and a negative value for timezones behind UTC (e.g., -420 for America/Denver).
   * Automatically handles DST transitions.
   */
  getOffset(timezone: string, epochMs: number): number;

  /**
   * List all available IANA timezone names.
   */
  list(): string[];
}

declare var Timezone: Timezone;

export { Timezone };
```

### Example

```javascript
import { Timezone } from "llrt:timezone";

// Get current offset for Denver (handles DST automatically)
const offset = Timezone.getOffset("America/Denver", Date.now());
// Returns -420 (UTC-7) in winter, -360 (UTC-6) in summer

// Check DST transition
const beforeDst = new Date("2024-03-09T12:00:00Z").getTime();
const afterDst = new Date("2024-03-11T12:00:00Z").getTime();
console.log(Timezone.getOffset("America/Denver", beforeDst)); // -420
console.log(Timezone.getOffset("America/Denver", afterDst)); // -360

// List all available timezones
const zones = Timezone.list();
```

### Intl.DateTimeFormat

LLRT provides a minimal `Intl.DateTimeFormat` implementation focused on timezone support. This enables libraries like dayjs to work with timezone conversions transparently.

```javascript
// Create a formatter for a specific timezone
const formatter = new Intl.DateTimeFormat("en-US", {
  timeZone: "America/Denver",
  hour12: false,
  year: "numeric",
  month: "2-digit",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
});

// Format a date
const date = new Date("2022-03-02T15:45:34Z");
console.log(formatter.format(date)); // "03/02/2022, 08:45:34"

// Get formatted parts
const parts = formatter.formatToParts(date);
// [{ type: "month", value: "03" }, { type: "literal", value: "/" }, ...]

// Get resolved options
const options = formatter.resolvedOptions();
console.log(options.timeZone); // "America/Denver"
```

### Date.prototype.toLocaleString with timezone

`Date.prototype.toLocaleString` is enhanced to support the `timeZone` option:

```javascript
const date = new Date("2022-03-02T15:45:34Z");

// Convert to Denver time
console.log(date.toLocaleString("en-US", { timeZone: "America/Denver" }));
// "03/02/2022, 8:45:34 AM"

// Convert to Tokyo time
console.log(date.toLocaleString("en-US", { timeZone: "Asia/Tokyo" }));
// "03/03/2022, 12:45:34 AM"
```

### Using with dayjs

The timezone module enables dayjs timezone support without polyfills:

```javascript
const dayjs = require("dayjs");
const utc = require("dayjs/plugin/utc");
const timezone = require("dayjs/plugin/timezone");

dayjs.extend(utc);
dayjs.extend(timezone);

// Convert between timezones
const date = dayjs("2022-03-02T15:45:34Z");
console.log(date.tz("America/Denver").format()); // "2022-03-02T08:45:34-07:00"
console.log(date.tz("Asia/Tokyo").format()); // "2022-03-03T00:45:34+09:00"

// Get start of day in a specific timezone
const denver = date.tz("America/Denver");
console.log(denver.startOf("day").format()); // "2022-03-02T00:00:00-07:00"
```

## llrt:qjs

```typescript
interface MemoryInfo {
  malloc_size: number;
  malloc_limit: number;
  memory_used_size: number;
  malloc_count: number;
  memory_used_count: number;
  atom_count: number;
  atom_size: number;
  str_count: number;
  str_size: number;
  obj_count: number;
  obj_size: number;
  prop_count: number;
  prop_size: number;
  shape_count: number;
  shape_size: number;
  js_func_count: number;
  js_func_size: number;
  js_func_code_size: number;
  js_func_pc2line_count: number;
  js_func_pc2line_size: number;
  c_func_count: number;
  array_count: number;
  fast_array_count: number;
  fast_array_elements: number;
  binary_object_count: number;
  binary_object_size: number;
}
export function ComputeMemoryUsage(): MemoryInfo;
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

# Web Platform API

## CONSOLE

[Console](https://developer.mozilla.org/en-US/docs/Web/API/console)

## DOM

[AbortController](https://developer.mozilla.org/en-US/docs/Web/API/AbortController)

[AbortSignal](https://developer.mozilla.org/en-US/docs/Web/API/AbortSignal)

[CustomEvent](https://developer.mozilla.org/en-US/docs/Web/API/CustomEvent)

[Event](https://developer.mozilla.org/en-US/docs/Web/API/Event)

[EventTarget](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget)

## ECMASCRIPT

[globalThis](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/globalThis)

## ENCODING

[TextDecoder](https://developer.mozilla.org/en-US/docs/Web/API/TextDecoder)

[TextEncoder](https://developer.mozilla.org/en-US/docs/Web/API/TextEncoder)

## FETCH

[Headers](https://developer.mozilla.org/en-US/docs/Web/API/Headers)

[Request](https://developer.mozilla.org/en-US/docs/Web/API/Request)

[Response](https://developer.mozilla.org/en-US/docs/Web/API/Response)

[fetch](https://developer.mozilla.org/en-US/docs/Web/API/Headers)

> [!IMPORTANT]
> There are some differences with the [WHATWG standard](https://fetch.spec.whatwg.org). Mainly browser specific behavior is removed:
>
> - `keepalive` is always true
> - `request.body` can only be `string`, `Array`, `ArrayBuffer` or `Uint8Array`
> - `response.body` returns `null`. Use `response.text()`, `response.json()` etc
> - `mode`, `credentials`, `referrerPolicy`, `priority`, `cache` is not available/applicable

## FILEAPI

[Blob](https://developer.mozilla.org/en-US/docs/Web/API/Blob)

[File](https://developer.mozilla.org/en-US/docs/Web/API/File)

## HR-TIME

[performance.now](https://developer.mozilla.org/en-US/docs/Web/API/Performance/now)

[performance.timeOrigin](https://developer.mozilla.org/en-US/docs/Web/API/Performance/timeOrigin)

## HTML

[atob](https://developer.mozilla.org/en-US/docs/Web/API/atob)

[btoa](https://developer.mozilla.org/en-US/docs/Web/API/btoa)

[clearInterval](https://developer.mozilla.org/en-US/docs/Web/API/Window/clearInterval)

[clearTimeout](https://developer.mozilla.org/en-US/docs/Web/API/Window/clearTimeout)

[navigator](https://developer.mozilla.org/en-US/docs/Web/API/Window/navigator)

[queueMicrotask](https://developer.mozilla.org/en-US/docs/Web/API/Window/queueMicrotask)

[setInterval](https://developer.mozilla.org/en-US/docs/Web/API/Window/setInterval)

[setTimeout](https://developer.mozilla.org/en-US/docs/Web/API/Window/setTimeout)

[structuredClone](https://developer.mozilla.org/en-US/docs/Web/API/Window/structuredClone)

[userAgent](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/userAgent)

## STREAMS

[ByteLengthQueuingStrategy](https://developer.mozilla.org/en-US/docs/Web/API/ByteLengthQueuingStrategy)

[CountQueuingStrategy](https://developer.mozilla.org/en-US/docs/Web/API/CountQueuingStrategy)

[ReadableByteStreamController](https://developer.mozilla.org/en-US/docs/Web/API/ReadableByteStreamController)

[ReadableStream](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream)

[ReadableStreamBYOBReader](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStreamBYOBReader)

[ReadableStreamBYOBRequest](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStreamBYOBRequest)

[ReadableStreamDefaultController](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStreamDefaultController)

[ReadableStreamDefaultReader](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStreamDefaultReader)

[WritableStream](https://developer.mozilla.org/en-US/docs/Web/API/WritableStream)

[WritableStreamDefaultController](https://developer.mozilla.org/en-US/docs/Web/API/WritableStreamDefaultController)

[WritableStreamDefaultWriter](https://developer.mozilla.org/en-US/docs/Web/API/WritableStreamDefaultWriter)

## URL

[URL](https://developer.mozilla.org/en-US/docs/Web/API/URL)

[URLSearchParams](https://developer.mozilla.org/en-US/docs/Web/API/URLSearchParams)

## WEBCRYPTO

[Crypto](https://developer.mozilla.org/en-US/docs/Web/API/Crypto)

[CryptoKey](https://developer.mozilla.org/en-US/docs/Web/API/CryptoKey)

[SubtleCrypto](https://developer.mozilla.org/en-US/docs/Web/API/SubtleCrypto)

## WEBIDL

[DOMException](https://developer.mozilla.org/en-US/docs/Web/API/DOMException)

## XHR

[FormData](https://developer.mozilla.org/en-US/docs/Web/API/FormData)
