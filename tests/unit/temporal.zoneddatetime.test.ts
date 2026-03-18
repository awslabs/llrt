describe("Temporal.ZonedDateTime", () => {
  describe("creation and parsing", () => {
    it("can be created from an RFC 9557 string", () => {
      const zdt1 = Temporal.ZonedDateTime.from(
        "2021-08-19T17:01:00+01:00[Europe/London]"
      );
      expect(zdt1.toString()).toContain(
        "2021-08-19T17:01:00+01:00[Europe/London]"
      );
    });

    it("can be created from an own object", () => {
      const zdt1 = Temporal.ZonedDateTime.from(
        "2021-08-19T17:01:00+01:00[Europe/London]"
      );
      const zdt2 = Temporal.ZonedDateTime.from(zdt1);
      expect(zdt1).toEqual(zdt2);
    });

    it("can be created from an object containing properties", () => {
      const zdt1 = Temporal.ZonedDateTime.from({
        timeZone: "America/Los_Angeles",
        year: 1995,
        month: 12,
        day: 7,
        hour: 3,
        minute: 24,
        second: 30,
        millisecond: 0,
        microsecond: 3,
        nanosecond: 500,
      });
      expect(zdt1.toString()).toEqual(
        "1995-12-07T03:24:30.0000035-08:00[America/Los_Angeles]"
      );
    });
  });

  describe("arithmetic methods", () => {
    const zdt1 = Temporal.ZonedDateTime.from("2020-01-01T12:00:00+00:00[UTC]");
    const zdt2 = Temporal.ZonedDateTime.from("2020-01-02T12:00:00+00:00[UTC]");

    it("add() advances the date/time", () => {
      const later = zdt1.add({ days: 1 });
      expect(later.day).toBe(2);
    });

    it("compare() method works for two ZonedDateTime objects", () => {
      expect(Temporal.ZonedDateTime.compare(zdt1, zdt2)).toBe(-1);
      expect(Temporal.ZonedDateTime.compare(zdt2, zdt1)).toBe(1);
      expect(Temporal.ZonedDateTime.compare(zdt1, zdt1)).toBe(0);
    });

    it("equals() verifies if two ZonedDateTime are equal", () => {
      const same = Temporal.ZonedDateTime.from("2020-01-01T12:00:00[UTC]");
      expect(zdt1.equals(same)).toBe(true);
    });

    it("getTimeZoneTransition() returns the next and previous timezone offset transitions", () => {
      const dt = Temporal.ZonedDateTime.from(
        "2024-01-01T00-05:00[America/New_York]"
      );
      const transition = dt.getTimeZoneTransition("next");
      expect(transition.toString()).toBe(
        "2024-03-10T03:00:00-04:00[America/New_York]"
      );

      const transition2 = transition.getTimeZoneTransition("next");
      expect(transition2.toString()).toBe(
        "2024-11-03T01:00:00-05:00[America/New_York]"
      );

      const transition3 = dt.getTimeZoneTransition("previous");
      expect(transition3.toString()).toBe(
        "2023-11-05T01:00:00-05:00[America/New_York]"
      );

      const dt2 = Temporal.ZonedDateTime.from("2024-01-01T00Z[UTC]");
      expect(dt2.getTimeZoneTransition("next")).toBe(null);
    });

    it("round() supports various forms, roundingMode, and roundingIncrement", () => {
      const zFrac = zdt1.add({ milliseconds: 500 });
      const rounded = zFrac.round({ smallestUnit: "second" });
      expect(rounded.second).toBe(1);
      expect(rounded.offset).toBe("+00:00");
      expect(rounded.timeZoneId).toBe("UTC");

      const truncated = zFrac.round({
        smallestUnit: "second",
        roundingMode: "trunc",
      });
      expect(truncated.second).toBe(0);

      const zMulti = zdt1.add({ minutes: 7, seconds: 15 });
      const str = zMulti.round("minute");
      expect(str.minute).toBe(7);
      expect(str.second).toBe(0);

      const inc = zMulti.round({
        smallestUnit: "minute",
        roundingIncrement: 15,
      });
      expect(inc.minute).toBe(0);

      const incHour = zMulti.round({
        smallestUnit: "hour",
        roundingIncrement: 2,
      });
      expect(incHour.hour).toBe(12);
    });

    it("since() returns correct duration", () => {
      const d = zdt2.since(zdt1);

      expect(d.sign).toBe(1);
      expect(d.years).toBe(0);
      expect(d.months).toBe(0);
      expect(d.days).toBe(0);
      expect(d.hours).toBe(24);
      expect(d.minutes).toBe(0);
      expect(d.seconds).toBe(0);
      expect(d.milliseconds).toBe(0);
      expect(d.microseconds).toBe(0);
      expect(d.nanoseconds).toBe(0);
    });

    it("subtract() decrements by a duration", () => {
      const earlier = zdt1.subtract({ hours: 1 });
      expect(earlier.hour).toBeLessThan(zdt1.hour);
    });

    it("toInstant() returns a Temporal.Instant", () => {
      const inst = zdt1.toInstant();
      expect(inst).toBeInstanceOf(Temporal.Instant);
    });

    it("toPlainDate() returns a Temporal.PlainDate", () => {
      const pd1 = zdt1.toPlainDate();
      expect(pd1).toBeInstanceOf(Temporal.PlainDate);
      expect(pd1.toString()).toBe("2020-01-01");
    });

    it("toPlainDateTime() returns a Temporal.PlainDateTime", () => {
      const pdt1 = zdt1.toPlainDateTime();
      expect(pdt1).toBeInstanceOf(Temporal.PlainDateTime);
      expect(pdt1.toString()).toBe("2020-01-01T12:00:00");
    });

    it("toPlainTime() returns a Temporal.PlainTime", () => {
      const pt1 = zdt1.toPlainTime();
      expect(pt1).toBeInstanceOf(Temporal.PlainTime);
      expect(pt1.toString()).toBe("12:00:00");
    });

    it("toString() returns an ISO string representation", () => {
      const str = zdt1.toString();
      expect(typeof str).toBe("string");
      expect(str).toBe("2020-01-01T12:00:00+00:00[UTC]");
    });

    it("until() returns correct duration", () => {
      const d = zdt1.until(zdt2);

      expect(d.sign).toBe(1);
      expect(d.years).toBe(0);
      expect(d.months).toBe(0);
      expect(d.days).toBe(0);
      expect(d.hours).toBe(24);
      expect(d.minutes).toBe(0);
      expect(d.seconds).toBe(0);
      expect(d.milliseconds).toBe(0);
      expect(d.microseconds).toBe(0);
      expect(d.nanoseconds).toBe(0);
    });

    it("valueOf() throws a TypeError", () => {
      expect(() => {
        // eslint-disable-next-line @typescript-eslint/no-unused-expressions
        +zdt1;
      }).toThrow();
    });

    it("with() method returns a new ZonedDateTime with updated fields", () => {
      const updated = zdt1.with({ day: 15 });
      expect(updated.day).toBe(15);
      expect(updated.month).toBe(zdt1.month);
    });

    it("withTimeZone() method returns a new ZonedDateTime with replaceing timezone", () => {
      const updated = zdt1.withTimeZone("Asia/Tokyo");
      expect(updated.toString()).toBe("2020-01-01T21:00:00+09:00[Asia/Tokyo]");
    });
  });

  describe("properties", () => {
    const zdt = Temporal.ZonedDateTime.from("2020-01-01T12:00:00[UTC]");

    it("should have correct date and time properties", () => {
      expect(zdt.year).toBe(2020);
      expect(zdt.month).toBe(1);
      expect(zdt.day).toBe(1);
      expect(zdt.hour).toBe(12);
      expect(zdt.minute).toBe(0);
      expect(zdt.second).toBe(0);
      expect(zdt.millisecond).toBe(0);
      expect(zdt.offset).toBe("+00:00");

      expect(typeof zdt.epochMilliseconds).toBe("number");
      expect(zdt.epochMilliseconds).toBeGreaterThan(0);
      expect(typeof zdt.epochNanoseconds).toBe("bigint");
      expect(zdt.epochNanoseconds).toBeGreaterThan(0n);

      expect(zdt.dayOfYear).toBe(1);
      expect(zdt.daysInMonth).toBe(31);
      expect(zdt.daysInYear).toBe(366);
      expect(zdt.inLeapYear).toBe(true);
    });

    it("has correct toStringTag", () => {
      expect(Object.prototype.toString.call(zdt)).toBe(
        "[object Temporal.ZonedDateTime]"
      );
    });
  });
});
