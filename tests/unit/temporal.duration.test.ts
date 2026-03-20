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
