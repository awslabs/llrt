# llrt_intl

Minimal internationalization support for LLRT. Provides a subset of `Intl` functionality focused on timezone support.

## Features

- **Intl.DateTimeFormat** - Minimal implementation supporting `format()`, `formatToParts()`, and `resolvedOptions()`
- **Date.prototype.toLocaleString** - Enhanced to support the `timeZone` option
- **dayjs compatibility** - Enables the dayjs timezone plugin without polyfills

## API

### `Intl.DateTimeFormat`

Minimal implementation for timezone-aware date formatting.

### `Date.prototype.toLocaleString`

Enhanced to support the `timeZone` option for timezone conversion.

## Examples

### Intl.DateTimeFormat

```javascript
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

const date = new Date("2022-03-02T15:45:34Z");
console.log(formatter.format(date)); // "03/02/2022, 08:45:34"
```

### Using with dayjs

```javascript
const dayjs = require("dayjs");
const utc = require("dayjs/plugin/utc");
const timezone = require("dayjs/plugin/timezone");

dayjs.extend(utc);
dayjs.extend(timezone);

const date = dayjs("2022-03-02T15:45:34Z");
console.log(date.tz("America/Denver").format()); // "2022-03-02T08:45:34-07:00"
console.log(date.tz("Asia/Tokyo").format()); // "2022-03-03T00:45:34+09:00"
```
