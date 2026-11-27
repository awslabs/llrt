describe("Constructor", () => {
  describe("Temporal.Duration()", () => {
    it("Using Temporal.Duration()", () => {
      const d = new Temporal.Duration(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
      expect(d.toString()).toBe("P1Y2M3W4DT5H6M7.00800901S");
    });
  });
});

describe("Static methods", () => {
  describe("Temporal.Duration.compare()", () => {
    it("Using Temporal.Instant.compare()", () => {
      const d1 = Temporal.Duration.from({ hours: 1, minutes: 30 });
      const d2 = Temporal.Duration.from({ minutes: 100 });
      expect(Temporal.Duration.compare(d1, d2)).toBe(-1);

      const d3 = Temporal.Duration.from({ hours: 2 });
      const d4 = Temporal.Duration.from({ minutes: 110 });
      expect(Temporal.Duration.compare(d3, d4)).toBe(1);

      const d5 = Temporal.Duration.from({ hours: 1, minutes: 30 });
      const d6 = Temporal.Duration.from({ seconds: 5400 });
      expect(Temporal.Duration.compare(d5, d6)).toBe(0);
    });

    it("Comparing calendar durations", () => {
      const d1 = Temporal.Duration.from({ days: 31 });
      const d2 = Temporal.Duration.from({ months: 1 });

      expect(
        Temporal.Duration.compare(d1, d2, {
          relativeTo: Temporal.PlainDate.from("2021-01-01"), // ISO 8601 calendar
        })
      ).toBe(0);

      expect(
        Temporal.Duration.compare(d1, d2, {
          relativeTo: Temporal.PlainDate.from("2021-02-01"), // ISO 8601 calendar
        })
      ).toBe(1);
    });

    it("Accepts duration strings and property bags in either argument", () => {
      const duration = Temporal.Duration.from({ minutes: 90 });
      expect(Temporal.Duration.compare({ hours: 1 }, duration)).toBe(-1);
      expect(Temporal.Duration.compare(duration, { hours: 1 })).toBe(1);
      expect(
        Temporal.Duration.compare("PT90M", { hours: 1, minutes: 30 })
      ).toBe(0);
      expect(() => Temporal.Duration.compare(duration, "invalid")).toThrow(
        RangeError
      );
      expect(() => Temporal.Duration.compare(null, duration)).toThrow(
        TypeError
      );
      expect(() => Temporal.Duration.compare({}, duration)).toThrow(TypeError);
      expect(() => Temporal.Duration.compare({ hours: NaN }, duration)).toThrow(
        RangeError
      );
      expect(() => Temporal.Duration.compare({ hours: 1.5 }, duration)).toThrow(
        RangeError
      );
      expect(
        Temporal.Duration.compare({ hours: "1", minutes: null }, "PT1H")
      ).toBe(0);
      expect(() =>
        Temporal.Duration.compare(
          {
            get hours() {
              throw new Error("duration getter");
            },
          },
          duration
        )
      ).toThrow("duration getter");
    });

    it("Accounts for time zone transitions when comparing days and hours", () => {
      const day = Temporal.Duration.from({ days: 1 });
      const hours = Temporal.Duration.from({ hours: 24 });
      const fall = Temporal.ZonedDateTime.from(
        "2019-11-03T00:00[America/Vancouver]"
      );
      const spring = Temporal.ZonedDateTime.from(
        "2019-03-10T00:00[America/Vancouver]"
      );

      expect(Temporal.Duration.compare(day, hours, { relativeTo: fall })).toBe(
        1
      );
      expect(
        Temporal.Duration.compare(day, hours, { relativeTo: spring })
      ).toBe(-1);

      const start = Temporal.ZonedDateTime.from(
        "1999-10-29T01:00[America/Vancouver]"
      );
      const yearAndDay = Temporal.Duration.from({ years: 1, days: 1 });
      const yearAndHours = Temporal.Duration.from({ years: 1, hours: 25 });
      expect(
        Temporal.Duration.compare(yearAndDay, yearAndHours, {
          relativeTo: start,
        })
      ).toBe(0);
    });

    it("Accepts strings and property bags as relativeTo", () => {
      const day = Temporal.Duration.from({ days: 1 });
      const hours = Temporal.Duration.from({ hours: 24 });
      expect(
        Temporal.Duration.compare(day, hours, {
          relativeTo: "2019-11-03T00:00[America/Vancouver]",
        })
      ).toBe(1);
      expect(
        Temporal.Duration.compare(day, hours, {
          relativeTo: {
            year: 2019,
            month: 11,
            day: 3,
            timeZone: "America/Vancouver",
          },
        })
      ).toBe(1);

      const month = Temporal.Duration.from({ months: 1 });
      const days = Temporal.Duration.from({ days: 31 });
      expect(
        Temporal.Duration.compare(days, month, { relativeTo: "2021-01-01" })
      ).toBe(0);
      expect(
        Temporal.Duration.compare(days, month, {
          relativeTo: "2021-01-01[foo=bar]",
        })
      ).toBe(0);
      expect(
        Temporal.Duration.compare(days, month, {
          relativeTo: { year: 2021, month: 2, day: 1 },
        })
      ).toBe(1);
    });

    it("Propagates relativeTo errors", () => {
      const hour = Temporal.Duration.from({ hours: 1 });
      const error = new Error("relativeTo getter");
      expect(() =>
        Temporal.Duration.compare(hour, hour, {
          get relativeTo() {
            throw error;
          },
        })
      ).toThrow("relativeTo getter");
      expect(() =>
        Temporal.Duration.compare(hour, hour, { relativeTo: "invalid" })
      ).toThrow(RangeError);
      expect(() =>
        Temporal.Duration.compare(hour, hour, { relativeTo: null })
      ).toThrow(TypeError);
    });
  });

  describe("Temporal.Duration.from()", () => {
    it("Creating a duration from an object", () => {
      const d1 = Temporal.Duration.from({ hours: 1, minutes: 30 });
      expect(d1.toString()).toBe("PT1H30M");

      const d2 = Temporal.Duration.from({ months: 1, days: 2 });
      expect(d2.toString()).toBe("P1M2D");

      // Uncommon because unbalanced, but valid
      const unbalanced = Temporal.Duration.from({
        hours: 100,
        minutes: 100,
        seconds: 100,
      });
      expect(unbalanced.toString()).toBe("PT100H100M100S");

      const neg = Temporal.Duration.from({ hours: -1, minutes: -30 });
      expect(neg.toString()).toBe("-PT1H30M");
    });

    it("Creating a duration from a string", () => {
      const d = Temporal.Duration.from("P1Y2M3W4DT5H6M7.00800901S");
      expect(d.hours).toBe(5);
    });

    it("Creating a duration from another duration", () => {
      const d1 = Temporal.Duration.from({ hours: 1, minutes: 30 });
      const d2 = Temporal.Duration.from(d1);
      expect(d2.toString()).toBe("PT1H30M");
    });
  });
});

describe("RelativeTo offset property bags", () => {
  it("uses exact offsets for rounding and totaling", () => {
    const duration = Temporal.Duration.from({ years: 1, hours: 24 });
    const relativeTo = {
      year: 1970,
      month: 1,
      day: 1,
      timeZone: "+00:45",
      offset: "+00:45:00.000000000",
    };

    for (const offset of [
      "+00:45",
      "+0045",
      "+004500",
      "+00:45:00.000000000",
      "+004500,0",
    ]) {
      const options = { ...relativeTo, offset };
      expect(
        duration.round({ largestUnit: "years", relativeTo: options }).toString()
      ).toBe("P1Y1D");
      expect(duration.total({ unit: "days", relativeTo: options })).toBe(366);
    }

    for (const offset of [
      "+00:45:00.000000001",
      "+00:44:59.999999999",
      "+00:45:00.",
      "+00:45:00.0000000000",
      "+00:4500",
      "+00:46",
    ]) {
      const options = { ...relativeTo, offset };
      expect(() =>
        duration.round({ largestUnit: "years", relativeTo: options })
      ).toThrow(RangeError);
      expect(() =>
        duration.total({ unit: "days", relativeTo: options })
      ).toThrow(RangeError);
    }
  });

  it("selects the offset-matching candidate in a time zone fold", () => {
    const duration = Temporal.Duration.from({ days: 1 });
    const relativeTo = {
      year: 1952,
      month: 10,
      day: 15,
      hour: 23,
      minute: 59,
      second: 59,
      timeZone: "Pacific/Niue",
      offset: "-11:19:40",
    };
    for (const [offset, seconds] of [
      ["-11:19:40", 86420],
      ["-11:20:00", 86400],
    ] as const) {
      const options = { ...relativeTo, offset };
      expect(
        duration.round({ largestUnit: "seconds", relativeTo: options }).seconds
      ).toBe(seconds);
      expect(duration.total({ unit: "seconds", relativeTo: options })).toBe(
        seconds
      );
    }

    const gap = {
      year: 2019,
      month: 3,
      day: 10,
      hour: 2,
      minute: 30,
      timeZone: "America/Vancouver",
      offset: "-08:00",
    };
    expect(() => duration.total({ unit: "seconds", relativeTo: gap })).toThrow(
      RangeError
    );
  });
});

describe("Instance methods", () => {
  describe("Temporal.Duration.prototype.abs()", () => {
    it("Using abs()", () => {
      const d1 = Temporal.Duration.from({ hours: 1, minutes: 30 });
      const d2 = Temporal.Duration.from({ hours: -1, minutes: -30 });

      expect(d1.abs().toString()).toBe("PT1H30M");
      expect(d2.abs().toString()).toBe("PT1H30M");
    });
  });

  describe("Temporal.Duration.prototype.add()", () => {
    it("Using add()", () => {
      const d1 = Temporal.Duration.from({ hours: 1, minutes: 30 });
      const d2 = Temporal.Duration.from({ hours: -1, minutes: -20 });

      const d3 = d1.add(d2);
      expect(d3.toString()).toBe("PT10M");
    });

    it("Adding calendar durations", () => {
      const d1 = Temporal.Duration.from({ days: 1 });
      const d2 = Temporal.Duration.from({ months: 1 });

      expect(() => {
        d1.add(d2); // RangeError: for calendar duration arithmetic, use date arithmetic relative to a starting point
      }).toThrow();

      const start = Temporal.PlainDateTime.from("2022-01-01T00:00"); // ISO 8601 calendar
      const result = start.add(d1).add(d2).since(start);
      expect(result.toString()).toBe("P32D");
    });
  });

  describe("Temporal.Duration.prototype.negated()", () => {
    it("Using negated()", () => {
      const d1 = Temporal.Duration.from({ hours: 1, minutes: 30 });
      const d2 = Temporal.Duration.from({ hours: -1, minutes: -30 });

      expect(d1.negated().toString()).toBe("-PT1H30M");
      expect(d2.negated().toString()).toBe("PT1H30M");
    });
  });

  describe("Temporal.Duration.prototype.round()", () => {
    it("Rounding off small units", () => {
      const duration = Temporal.Duration.from({
        hours: 1,
        minutes: 30,
        seconds: 15,
      });
      const roundedDuration = duration.round("minutes");
      expect(roundedDuration.toString()).toBe("PT1H30M");
    });

    it("Avoiding larger units", () => {
      const duration = Temporal.Duration.from({
        days: 3,
        hours: 1,
        minutes: 41,
        seconds: 5,
      });
      const roundedDuration = duration.round({ largestUnit: "hours" });
      // expect(
      //   `Time spent on this problem: ${roundedDuration.toLocaleString("en-US", { style: "digital" })}`,
      // );
      // Time spent on this problem: 73:41:05
      expect(roundedDuration.toString()).toBe("PT73H41M5S");
    });

    it("Rounding to a whole number of hours", () => {
      const duration = Temporal.Duration.from({
        days: 1,
        hours: 1,
        minutes: 30,
      });
      const roundedDuration = duration.round({
        largestUnit: "hours",
        smallestUnit: "hours",
        roundingMode: "floor",
      });
      expect(roundedDuration.hours).toBe(25);
    });

    it("Rounding by 15-minute increments", () => {
      const duration = Temporal.Duration.from({ hours: 1, minutes: 17 });
      const roundedDuration = duration.round({
        smallestUnit: "minutes",
        roundingIncrement: 15,
      });
      // expect(
      //   `The queue will take approximately ${roundedDuration.toLocaleString("en-US")}`,
      // );
      // The queue will take approximately 1 hr, 15 min
      expect(roundedDuration.toString()).toBe("PT1H15M");
    });

    it("Resolving calendar durations", () => {
      const duration = Temporal.Duration.from({ months: 1, days: 1, hours: 1 });
      const roundedDuration = duration.round({
        largestUnit: "days",
        smallestUnit: "days",
        relativeTo: Temporal.PlainDateTime.from("2022-01-01"),
      });
      expect(roundedDuration.toString()).toBe("P32D");
    });

    it("Accounts for time zone transitions when rounding", () => {
      const duration = Temporal.Duration.from({ days: 1 });
      const relativeTo = Temporal.ZonedDateTime.from(
        "2019-11-03T00:00[America/Vancouver]"
      );
      expect(duration.round({ largestUnit: "hours", relativeTo }).hours).toBe(
        25
      );
      expect(
        duration.round({
          largestUnit: "hours",
          relativeTo: "2019-11-03T00:00[America/Vancouver]",
        }).hours
      ).toBe(25);
    });
  });

  describe("Temporal.Duration.prototype.subtract()", () => {
    it("Using subtract()", () => {
      const d1 = Temporal.Duration.from({ hours: 1, minutes: 30 });
      const d2 = Temporal.Duration.from({ hours: -1, minutes: -20 });

      const d3 = d1.subtract(d2);
      expect(d3.toString()).toBe("PT2H50M");
    });
  });

  describe("Temporal.Duration.prototype.toJSON()", () => {
    it("Using toJSON()", () => {
      const duration = Temporal.Duration.from({
        hours: 1,
        minutes: 30,
        seconds: 15,
      });
      const durationStr = duration.toJSON();
      expect(durationStr).toBe("PT1H30M15S");
      const d2 = Temporal.Duration.from(durationStr);
    });

    it("JSON serialization and parsing", () => {
      const duration = Temporal.Duration.from({
        hours: 1,
        minutes: 30,
        seconds: 15,
      });
      const jsonStr = JSON.stringify({ data: duration });
      expect(jsonStr).toBe('{"data":"PT1H30M15S"}');
      const obj = JSON.parse(jsonStr, (key, value) => {
        if (key === "data") {
          return Temporal.Duration.from(value);
        }
        return value;
      });
    });
  });

  describe("Temporal.Duration.prototype.toString()", () => {
    it("Using toString()", () => {
      const duration = Temporal.Duration.from({
        hours: 1,
        minutes: 30,
        seconds: 15,
      });
      expect(duration.toString()).toBe("PT1H30M15S");

      // Stringification implicitly calls toString()
      expect(`${duration}`).toBe("PT1H30M15S");
    });
  });

  describe("Temporal.Duration.prototype.total()", () => {
    it("Using total()", () => {
      const d = Temporal.Duration.from({ hours: 1, minutes: 30 });

      expect(d.total("minutes")).toBe(90);
      expect(d.total("hours")).toBe(1.5);
    });

    it("Total of a calendar duration", () => {
      const d = Temporal.Duration.from({ months: 1 });

      expect(
        d.total({
          unit: "days",
          relativeTo: Temporal.PlainDate.from("2021-01-01"),
        })
      ).toBe(31);
    });

    it("Accounts for time zone transitions when totaling", () => {
      const duration = Temporal.Duration.from({ days: 1 });
      const relativeTo = Temporal.ZonedDateTime.from(
        "2019-11-03T00:00[America/Vancouver]"
      );
      expect(duration.total({ unit: "hours", relativeTo })).toBe(25);
      expect(
        duration.total({
          unit: "hours",
          relativeTo: {
            year: 2019,
            month: 11,
            day: 3,
            timeZone: "America/Vancouver",
          },
        })
      ).toBe(25);
    });
  });

  describe("Temporal.Duration.prototype.valueOf()", () => {
    it("Arithmetic and comparison operations on Temporal.Duration", () => {
      const duration1 = Temporal.Duration.from({ seconds: 3 });
      const duration2 = Temporal.Duration.from({ minutes: 1 });
      expect(() => {
        duration1 > duration2; // TypeError: can't convert Duration to primitive type
      }).toThrow();
      expect(duration1.total("seconds") > duration2.total("seconds")).toBe(
        false
      );
      expect(Temporal.Duration.compare(duration1, duration2)).toBe(-1);

      expect(() => {
        duration1 + duration2; // TypeError: can't convert Duration to primitive type
      }).toThrow();
      expect(duration1.total("seconds") + duration2.total("seconds")).toBe(63);
      expect(duration1.add(duration2).toString()).toBe("PT1M3S");
    });
  });

  describe("Temporal.Duration.prototype.with()", () => {
    it("Using with()", () => {
      function balanceMinutes(duration) {
        const { hours, minutes } = duration;
        const totalMinutes = hours * 60 + minutes;
        const balancedMinutes = totalMinutes % 60;
        const balancedHours = (totalMinutes - balancedMinutes) / 60;
        return duration.with({
          hours: balancedHours,
          minutes: balancedMinutes,
        });
      }

      const d1 = Temporal.Duration.from({
        hours: 100,
        minutes: 100,
        seconds: 100,
      });
      const d2 = balanceMinutes(d1);
      expect(d2.hours).toBe(101);
      expect(d2.minutes).toBe(40);
      expect(d2.seconds).toBe(100);
    });
  });
});

describe("Instance properties", () => {
  it("should have correct property values for weeks and all components", () => {
    const dur = Temporal.Duration.from({ weeks: 2, days: 3, hours: 5 });
    expect(dur.weeks).toBe(2);
    expect(dur.days).toBe(3);
    expect(dur.hours).toBe(5);
  });

  it("should report the correct sign for durations", () => {
    const pos = Temporal.Duration.from({ hours: 5 });
    expect(pos.sign).toBe(1);

    const neg = pos.negated();
    expect(neg.sign).toBe(-1);

    const zero = Temporal.Duration.from("PT0S");
    expect(zero.sign).toBe(0);
  });

  it("has correct toStringTag", () => {
    const dur = Temporal.Duration.from("PT0S");
    expect(Object.prototype.toString.call(dur)).toBe(
      "[object Temporal.Duration]"
    );
  });
});
