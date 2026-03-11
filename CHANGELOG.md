### Features
  - Added dgram (UDP socket) module support (@chessbyte, @richarddavison)
  - Added timezone support with minimal Intl.DateTimeFormat (@chessbyte, @richarddavison)
  - Added Symbol.toStringTag to Web API classes (@chessbyte)
  - Added Symbol.toStringTag to Crypto and SubtleCrypto (@chessbyte)
  - Implemented `symlink` and `symlinkSync` in fs module (@kyubisation, @richarddavison)
  - Exposed FormData in fetch (@nabetti1720)
  - Exposed `module.registerHooks()` (@nabetti1720, @richarddavison)
  - Exposed `llrt:qjs` module (@nabetti1720)
  - Added modular crypto with multiple backend options (@richarddavison)
  - Eliminated `ring` dependency from pure-rust crypto backend (@nabetti1720)
  - Exported Md5, Sha1, Sha256, Sha384, Sha512 hash classes (@richarddavison)
  - Improved performance interface compatibility in perf_hooks (@nabetti1720)
  - Simplified child_process signal handling and detachment (@richarddavison)
  - Added custom inspect functions for Map, Set, DataView, and ArrayBuffer (@richarddavison)
  - Added basic Agent support (@Sytten)
  - Transitioned from `chrono/chrono-tz` to `jiff` (@nabetti1720)

### Fixes
  - Fix Proxy object handling JSON and console modules
  - Fix S3 endpoint resolution issue in new SDK version (@richarddavison)
  - Fix suite hook handling in TestAgent (@richarddavison)
  - Handle secret param in Hash constructors for SDK signing (@richarddavison)
  - Fix Headers keys and values methods to return iterable values (@richarddavison)
  - Improve encoding handling in Hash and Hmac implementations (@richarddavison)
  - Fix Buffer from utf16le (@Sytten)
  - Buffer improvements (@Sytten)
  - Use Latin-1 encoding for atob() per WHATWG spec (@chessbyte, @richarddavison)
  - Fix buffer offsets when repeating subarray (@nabetti1720, @richarddavison)
  - Fix stream/web primordials used before initialization (@nabetti1720)
  - Correctly handle 'require' with parent directory specification (@nabetti1720, @richarddavison)
  - Register and run test hooks in correct order (@kyubisation, @richarddavison)
  - Fix & simplify json escape (@richarddavison)
  - Require primordials to be initialized from module once to avoid data race (@richarddavison)
  - Ignore init for global clients not in us-east-1 (@perpil)
  - Improve support for dns lookup (@Sytten)
  - Remove zstd WriteBuf dependency for byte slice conversion (@chessbyte)
  - Disable `mlkem` in rustls-graviola (@nabetti1720)
  - Optional webpki (@Sytten)

### Maintenance
  - Upgrade rquickjs to 0.11 (@nabetti1720)
  - Upgrade rquickjs to 0.10.0 (@mohebifar, @richarddavison)
  - Eliminate use of `jwalk` for directory recursion (@nabetti1720)
  - Update SDK dependencies and remove UUID module (@richarddavison)
  - Dependency upgrades

Thanks for all the reports and contributors

Full list of changes:
https://github.com/awslabs/llrt/compare/v0.7.0-beta...v0.8.1-beta
