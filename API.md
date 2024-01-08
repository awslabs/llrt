# API documentation

The long term goal for LLRT is to become 100% [Winter CG compliant](https://github.com/wintercg/admin/blob/main/proposals.md) and have a frictionless migration path from Node.js. Not every API from node will be supported.

## buffer

[alloc](https://nodejs.org/api/buffer.html#static-method-bufferallocsize-fill-encoding) _Encoding is ignored_

[from](https://nodejs.org/api/buffer.html#static-method-bufferfromarray)

[concat](https://nodejs.org/api/buffer.html#static-method-bufferconcatlist-totallength)

[byteLength](https://nodejs.org/api/buffer.html#static-method-bufferbytelengthstring-encoding)

Everything else inherited from [Uint8Array](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array)

## child_process

[spawn](https://nodejs.org/api/child_process.html#child_processspawncommand-args-options)

## crypto

[createHash](https://nodejs.org/api/crypto.html#cryptocreatehashalgorithm-options)

[createHmac](https://nodejs.org/api/crypto.html#cryptocreatehmacalgorithm-key-options)

[randomBytes](https://nodejs.org/api/crypto.html#cryptorandombytessize-callback)

[randomFill](https://nodejs.org/api/crypto.html#cryptorandomfillbuffer-offset-size-callback)

[randomFillSync](https://nodejs.org/api/crypto.html#cryptorandomfillsyncbuffer-offset-size)

[randomUUID](https://nodejs.org/api/crypto.html#cryptorandomuuidoptions)

## events

[EventEmitter](https://nodejs.org/api/events.html#class-eventemitter)

## fetch

Available globally

[fetch](https://developer.mozilla.org/en-US/docs/Web/API/fetch) _There are some differences with the [WHATWG standard](https://fetch.spec.whatwg.org). Mainly browser specific behavior is removed:_
* `keepalive` is always true
* `body` can only be `string`, `Array`, `ArrayBuffer` or `Uint8Array`
* `mode`, `credentials`,  `referrerPolicy`, `priority`, `cache` is not available/applicable

## fs/promises

[readdir](https://nodejs.org/api/fs.html#fspromisesreaddirpath-options)

[readFile](https://nodejs.org/api/fs.html#filehandlereadfileoptions)

[writeFile](https://nodejs.org/api/fs.html#fspromiseswritefilefile-data-options)

[mkdir](https://nodejs.org/api/fs.html#fsmkdirpath-options-callback)

[mkdtemp](https://nodejs.org/api/fs.html#fsmkdtempprefix-options-callback)

[rmdir](https://nodejs.org/api/fs.html#fsrmdirpath-options-callback)

[rm](https://nodejs.org/api/fs.html#fsrmpath-options-callback)

[stat](https://nodejs.org/api/fs.html#fsstatpath-options-callback)

[access](https://nodejs.org/api/fs.html#fsstatpath-options-callback)

[constants](https://nodejs.org/api/fs.html#file-access-constants)

## hex

```typescript
export function encode(value:string|Array|ArrayBuffer|Uint8Array):string
export function decode(value:string):Uint8Array
```

## module

[createRequire](https://nodejs.org/api/module.html#modulecreaterequirefilename) _Require is available from esm modules natively. This function is just for compatibility_

## os

[platform](https://nodejs.org/api/os.html#osplatform)

[tmpdir](https://nodejs.org/api/os.html#osplatform)

[release](https://nodejs.org/api/os.html#osrelease)

[type](https://nodejs.org/api/os.html#ostype)

## path

[dirname](https://nodejs.org/api/path.html#pathdirnamepath)

[basename](https://nodejs.org/api/path.html#pathbasenamepath-suffix)

[extname](https://nodejs.org/api/path.html#pathextnamepath)

[format](https://nodejs.org/api/path.html#pathformatpathobject)

[parse](https://nodejs.org/api/path.html#pathparsepath)

[join](https://nodejs.org/api/path.html#pathjoinpaths)

[resolve](https://nodejs.org/api/path.html#pathresolvepaths)

[normalize](https://nodejs.org/api/path.html#pathnormalizepath)

[isAbsolute](https://nodejs.org/api/path.html#pathisabsolutepath)

[delimiter](https://nodejs.org/api/path.html#pathdelimiter)

## timers

Also available globally

[setImmediate](https://nodejs.org/api/timers.html#setimmediatecallback-args)

[setInterval](https://nodejs.org/api/timers.html#setintervalcallback-delay-args)

[setTimeout](https://nodejs.org/api/timers.html#settimeoutcallback-delay-args)

[clearImmediate](https://nodejs.org/api/timers.html#clearimmediateimmediate)

[clearInterval](https://nodejs.org/api/timers.html#clearintervaltimeout)

[clearTimeout](https://nodejs.org/api/timers.html#cleartimeouttimeout)

## util

_Supported encodings:  hex, base64, utf8, iso88591_

[TextEncoder](https://nodejs.org/api/util.html#class-utiltextdecoder)

_Supports only `decode`_

[TextDecoder](https://nodejs.org/api/util.html#class-utiltextdecoder)

## uuid

```typescript

export const NIL:string

export function v1():string

export function v3(name:string, namespace:Array|Uint8Array|String):string

export function v4():string

export function v5(name:string, namespace:Array|Uint8Array|String):string

export function parse(value:string):Uint8Array

export function stringify(arr:Array|Uint8Array):string

export function validate(arr:string):boolean

export function version(arr:Array|Uint8Array):number
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

_These APIs uses native streams that is not 100% compatible with the Node.js Streams API._

[createConnection](https://nodejs.org/api/net.html#netcreateconnection)

[connect](https://nodejs.org/api/net.html#netconnect)

[createServer](https://nodejs.org/api/net.html#netcreateserveroptions-connectionlistener) _This module provides limited functionality useful for testing purposes. Serverless applications typically don't expose servers. Some server options are not supported:_
`highWaterMark`, `pauseOnConnect`, `keepAlive`, `noDelay`, `keepAliveInitialDelay`
