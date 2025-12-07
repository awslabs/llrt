# llrt_timezone

Lightweight timezone support for LLRT. Provides timezone offset calculations and a minimal `Intl.DateTimeFormat` implementation for dayjs and similar library compatibility.

## Features

- **Timezone offset calculations** - Get UTC offsets for any IANA timezone with DST support
- **Intl.DateTimeFormat** - Minimal implementation supporting `format()`, `formatToParts()`, and `resolvedOptions()`
- **Date.prototype.toLocaleString** - Enhanced to support the `timeZone` option
- **dayjs compatibility** - Enables the dayjs timezone plugin without polyfills

## API

### `Timezone.getOffset(timezone: string, epochMs: number): number`

Returns the UTC offset in minutes for the given timezone at the specified time.

### `Timezone.list(): string[]`

Returns a list of all available IANA timezone names.

### `Intl.DateTimeFormat`

Minimal implementation for timezone-aware date formatting.

### `Date.prototype.toLocaleString`

Enhanced to support the `timeZone` option for timezone conversion.

## Examples

### Basic Timezone API

```javascript
import { Timezone } from "llrt:timezone";

// Get offset for America/Denver at a specific time
const offset = Timezone.getOffset("America/Denver", Date.now());
console.log(offset); // -420 (UTC-7 in minutes) or -360 (UTC-6 during DST)

// List all available timezones
const timezones = Timezone.list();
```

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

## Note on ES Module imports

When using dayjs with LLRT, use CommonJS `require()` syntax. ES module imports of dayjs plugins may not work correctly due to UMD wrapper handling differences.
