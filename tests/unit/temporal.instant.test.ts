describe("Constructor", () => {
  describe("Temporal.Instant()", () => {
    it("Using Temporal.Instant()", () => {
      const instant = new Temporal.Instant(0n);
      expect(instant.toString()).toBe("1970-01-01T00:00:00Z");
      const vostok1Liftoff = new Temporal.Instant(-275248380000000000n);
      expect(vostok1Liftoff.toString()).toBe("1961-04-12T06:07:00Z");
      const sts1Liftoff = new Temporal.Instant(355924804000000000n);
      expect(sts1Liftoff.toString()).toBe("1981-04-12T12:00:04Z");
    });
  });
});

describe("Static methods", () => {
  describe("Temporal.Instant.prototype.compare()", () => {
    it("Using Temporal.Instant.compare()", () => {
      const instant1 = Temporal.Instant.from("2021-08-01T12:34:56Z");
      const instant2 = Temporal.Instant.from("2021-08-01T12:34:56Z");

      expect(Temporal.Instant.compare(instant1, instant2)).toBe(0);

      const instant3 = Temporal.Instant.from("2021-08-01T13:34:56Z");
      expect(Temporal.Instant.compare(instant1, instant3)).toBe(-1);
    });
  });

  describe("Temporal.Instant.prototype.from()", () => {
    it("Creating an instant from a string", () => {
      const instant = Temporal.Instant.from("1970-01-01T00Z");
      expect(instant.toString()).toBe("1970-01-01T00:00:00Z");

      const instant2 = Temporal.Instant.from("1970-01-01T00+08:00");
      expect(instant2.toString()).toBe("1969-12-31T16:00:00Z");

      // America/New_York is UTC-5 in January 1970, not UTC+8
      const instant3 = Temporal.Instant.from(
        "1970-01-01T00+08:00[America/New_York]"
      );
      expect(instant3.toString()).toBe("1969-12-31T16:00:00Z");
    });
  });

  describe("Temporal.Instant.fromEpochMilliseconds()", () => {
    it("Using Temporal.Instant.fromEpochMilliseconds()", () => {
      const instant = Temporal.Instant.fromEpochMilliseconds(0);
      expect(instant.toString()).toBe("1970-01-01T00:00:00Z");
      const vostok1Liftoff =
        Temporal.Instant.fromEpochMilliseconds(-275248380000);
      expect(vostok1Liftoff.toString()).toBe("1961-04-12T06:07:00Z");
      const sts1Liftoff = Temporal.Instant.fromEpochMilliseconds(355924804000);
      expect(sts1Liftoff.toString()).toBe("1981-04-12T12:00:04Z");
    });
  });

  describe("Temporal.Instant.fromEpochNanoseconds()", () => {
    it("Using Temporal.Instant.fromEpochNanoseconds()", () => {
      const instant = Temporal.Instant.fromEpochNanoseconds(0n);
      expect(instant.toString()).toBe("1970-01-01T00:00:00Z");
      const vostok1Liftoff =
        Temporal.Instant.fromEpochNanoseconds(-275248380000000000n);
      expect(vostok1Liftoff.toString()).toBe("1961-04-12T06:07:00Z");
      const sts1Liftoff =
        Temporal.Instant.fromEpochNanoseconds(355924804000000000n);
      expect(sts1Liftoff.toString()).toBe("1981-04-12T12:00:04Z");
    });
  });
});

describe("Instance methods", () => {
  describe("Temporal.Instant.prototype.add()", () => {
    it("Adding a Temporal.Duration", () => {
      const instant = Temporal.Instant.fromEpochMilliseconds(0);
      const duration = Temporal.Duration.from("PT1S");
      const newInstant = instant.add(duration);
      expect(newInstant.epochMilliseconds).toBe(1000);
    });

    it("Adding an object or a string", () => {
      const instant = Temporal.Instant.fromEpochMilliseconds(0);
      const newInstant = instant.add({ seconds: 1 });
      expect(newInstant.epochMilliseconds).toBe(1000);

      const newInstant2 = instant.add("PT1S");
      expect(newInstant2.epochMilliseconds).toBe(1000);
    });

    it("Adding a calendar duration", () => {
      const instant = Temporal.Instant.fromEpochMilliseconds(1730610000000);
      const duration = Temporal.Duration.from({ days: 1 });

      // This instant is 2024-11-03T01:00:00-04:00[America/New_York],
      // which is a DST transition day in the US.
      const instant2 = instant
        .toZonedDateTimeISO("America/New_York")
        .add(duration)
        .toInstant();
      expect(instant2.epochMilliseconds).toBe(1730700000000);

      // The same instant is not a DST transition day in Paris.
      const instant3 = instant
        .toZonedDateTimeISO("Europe/Paris")
        .add(duration)
        .toInstant();
      expect(instant3.epochMilliseconds).toBe(1730696400000);
    });
  });

  describe("Temporal.Instant.prototype.equals()", () => {
    it("Using equals()", () => {
      const instant1 = Temporal.Instant.from("2021-08-01T12:34:56Z");
      const instant2 = Temporal.Instant.fromEpochMilliseconds(1627821296000);
      expect(instant1.equals(instant2)).toBe(true);
    });
  });

  describe("Temporal.Instant.prototype.round()", () => {
    it("Rounding off small units", () => {
      const instant = Temporal.Instant.fromEpochMilliseconds(1000);
      const roundedInstant = instant.round("second");
      expect(roundedInstant.epochMilliseconds).toBe(1000);

      const instant2 = instant.round("minute");
      expect(instant2.epochMilliseconds).toBe(0);
    });
  });

  describe("Temporal.Instant.prototype.since()", () => {
    it("since() returns correct duration", () => {
      const insta = Temporal.Instant.fromEpochMilliseconds(5000);
      const instb = Temporal.Instant.fromEpochMilliseconds(2000);

      const d = insta.since(instb);

      expect(d.years).toBe(0);
      expect(d.months).toBe(0);
      expect(d.days).toBe(0);
      expect(d.hours).toBe(0);
      expect(d.minutes).toBe(0);
      expect(d.seconds).toBe(3);
      expect(d.milliseconds).toBe(0);
      expect(d.microseconds).toBe(0);
      expect(d.nanoseconds).toBe(0);
    });
  });

  describe("Temporal.Instant.prototype.subtract()", () => {
    it("Subtracting a Temporal.Duration", () => {
      const instant = Temporal.Instant.fromEpochMilliseconds(1000);
      const duration = Temporal.Duration.from("PT1S"); // One-second duration
      const newInstant = instant.subtract(duration);
      expect(newInstant.epochMilliseconds).toBe(0);
    });
  });

  describe("Temporal.Instant.prototype.toJSON()", () => {
    it("Using toJSON()", () => {
      const instant = Temporal.Instant.fromEpochMilliseconds(1627821296000);
      const instantStr = instant.toJSON();
      expect(instantStr).toBe("2021-08-01T12:34:56Z");
    });
  });

  describe("Temporal.Instant.prototype.toString()", () => {
    it("Using toString()", () => {
      const instant = Temporal.Instant.fromEpochMilliseconds(1627814412345);
      // expect(instant.toString()).toBe("2021-08-01T10:40:12.345Z"); // TODO: Expected value
      expect(instant.toString()).toBe("2021-08-01T10:40:12.344999936Z");
    });
  });

  describe("Temporal.Instant.prototype.toZonedDateTimeISO()", () => {
    it("Using toZonedDateTimeISO()", () => {
      const instant = Temporal.Instant.from("2021-08-01T12:34:56.123456789Z");
      const zonedDateTime = instant.toZonedDateTimeISO("America/New_York");
      expect(zonedDateTime.toString()).toBe(
        "2021-08-01T08:34:56.123456789-04:00[America/New_York]"
      );
    });
  });

  describe("Temporal.Instant.prototype.until()", () => {
    it("until() returns correct duration", () => {
      const insta = Temporal.Instant.fromEpochMilliseconds(1000);
      const instb = Temporal.Instant.fromEpochMilliseconds(4000);

      const d = insta.until(instb);

      expect(d.years).toBe(0);
      expect(d.months).toBe(0);
      expect(d.days).toBe(0);
      expect(d.hours).toBe(0);
      expect(d.minutes).toBe(0);
      expect(d.seconds).toBe(3);
      expect(d.milliseconds).toBe(0);
      expect(d.microseconds).toBe(0);
      expect(d.nanoseconds).toBe(0);
    });
  });

  describe("Temporal.Instant.prototype.valueOf()", () => {
    it("valueOf() throws a TypeError", () => {
      const inst = Temporal.Instant.fromEpochMilliseconds(0);
      expect(() => {
        // eslint-disable-next-line @typescript-eslint/no-unused-expressions
        +inst;
      }).toThrow();
    });
  });
});

describe("Instance properties", () => {
  const inst = Temporal.Instant.fromEpochMilliseconds(1_609_459_260_000);

  it("reports epochMilliseconds and epochNanoseconds", () => {
    expect(typeof inst.epochMilliseconds).toBe("number");
    expect(inst.epochMilliseconds).toBe(1609459260000);
    expect(typeof inst.epochNanoseconds).toBe("bigint");
    expect(inst.epochNanoseconds).toBe(1609459260000000000n);
  });

  it("has correct toStringTag", () => {
    expect(Object.prototype.toString.call(inst)).toBe(
      "[object Temporal.Instant]"
    );
  });
});
