### Features
  - Added support for zstd compression/decompression in zlib (@nabetti1720)
  - Expanded WPT testing scope (@nabetti1720)
  - Exposed webcrypto in crypto module (@nabetti1720)
  - Exposed rename in filesystem module (@ChausseBenjamin)
  - Added --executable flag to llrt compile (@kyubisation)

### Fixes
  - Correctly handle percent encoding in data-urls (@nabetti1720)
  - Remove BOM from body.text() and body.json() (@nabetti1720)
  - Correctly handle wildcards in package.json (@nabetti1720)
  - Correct beforeEach/afterEach calls in tests (@kyubisation)
  - Fix fetch redirection (@richarddavison)
  - Improved compatibility of bodyUsed property (@nabetti1720)
  - Fix trimming even if BOM character code does not match (@nabetti1720)
  - Check for valid URI in fetch (@nabetti1720)
  - Convert trailing space from opaque path in URL (@nabetti1720)
  - SDK connection warmup for global endpoints (@richarddavison)
  - Add support for node: prefix for remaining packages (@willfarrell)
  - Expose stream/promises by polyfill (@nabetti1720)
  - Support resolving modules when require path ends with a trailing slash (@nabetti1720)
  - Fix Top-Level-Await webcall in Lambda handler (@richarddavison)
  - Fix Response.json() static method does not correctly hold the body (@nabetti1720)

### Maintenance
  - Core and build cleanup
  - Dependency upgrades

Thanks for all the reports and contributors

Full list of changes:
https://github.com/awslabs/llrt/compare/v0.6.2-beta...v0.7.0-beta
