describe("Temporal.Instant", () => {
  describe("creation and parsing", () => {
    it("can be created from epoch milliseconds", () => {
      const inst = Temporal.Instant.fromEpochMilliseconds(0);
      expect(inst.toString()).toBe("1970-01-01T00:00:00Z");
    });

    it("can be created from an RFC 9557 string", () => {
      const inst = Temporal.Instant.from("2021-08-19T17:01:00Z");
      expect(inst.toString()).toContain("2021-08-19T17:01:00Z");
    });

    it("can be created from an own object", () => {
      const inst1 = Temporal.Instant.fromEpochMilliseconds(0);
      const inst2 = Temporal.Instant.from(inst1);
      expect(inst1).toEqual(inst2);
    });
  });

  describe("arithmetic methods", () => {
    const inst = Temporal.Instant.fromEpochMilliseconds(1000);
    const dur = Temporal.Duration.from({ seconds: 1 });

    it("add() increments by duration", () => {
      const later = inst.add(dur);
      expect(later.epochMilliseconds).toBe(2000n);
    });

    it("equals() and compare() behave as expected", () => {
      const same = Temporal.Instant.fromEpochMilliseconds(1000);
      expect(inst.equals(same)).toBe(true);
      expect(Temporal.Instant.compare(inst, same)).toBe(0);
    });

    it("fromEpochNanoseconds() creates from nanoseconds", () => {
      const inst =
        Temporal.Instant.fromEpochNanoseconds(1_609_459_260_000_000_000);
      expect(inst.epochNanoseconds).toBe(1_609_459_260_000_000_000n);
    });

    it("round() supports various forms, roundingMode, and roundingIncrement", () => {
      const inst1 = inst.add({ milliseconds: 500 });
      const rounded1 = inst1.round({ smallestUnit: "second" });
      expect(rounded1.epochMilliseconds).toBe(2000n);

      const truncated1 = inst1.round({
        smallestUnit: "second",
        roundingMode: "trunc",
      });
      expect(truncated1.epochMilliseconds).toBe(1000n);

      const inst2 = inst.add({ milliseconds: 122450 });
      const asString = inst2.round("second");
      expect(asString.epochMilliseconds).toBe(123000n);

      const inc = inst2.round({
        smallestUnit: "second",
        roundingIncrement: 30,
      });
      expect(inc.epochMilliseconds).toBe(120000n);
    });

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

    it("subtract() decrements by duration", () => {
      const earlier = inst.subtract(dur);
      expect(earlier.epochMilliseconds).toBe(0n);
    });

    it("toZonedDateTimeISO() converts to ZonedDateTime", () => {
      const inst = Temporal.Instant.fromEpochMilliseconds(0);
      const zdt = inst.toZonedDateTimeISO("UTC");
      expect(zdt).toBeInstanceOf(Temporal.ZonedDateTime);
      expect(zdt.timeZoneId).toBe("UTC");
    });

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

    it("valueOf() throws a TypeError", () => {
      const inst = Temporal.Instant.fromEpochMilliseconds(0);
      expect(() => {
        // eslint-disable-next-line @typescript-eslint/no-unused-expressions
        +inst;
      }).toThrow();
    });
  });

  describe("properties", () => {
    const inst = Temporal.Instant.fromEpochMilliseconds(1_609_459_260_000);

    it("reports epochMilliseconds and epochNanoseconds", () => {
      expect(typeof inst.epochMilliseconds).toBe("bigint");
      expect(inst.epochMilliseconds).toBe(1609459260000n);
      expect(typeof inst.epochNanoseconds).toBe("bigint");
      expect(inst.epochNanoseconds).toBe(1609459260000000000n);
    });

    it("has correct toStringTag", () => {
      expect(Object.prototype.toString.call(inst)).toBe(
        "[object Temporal.Instant]"
      );
    });
  });
});
