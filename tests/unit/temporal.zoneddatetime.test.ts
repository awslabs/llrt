describe("Temporal.ZonedDateTime", () => {
  describe("creation and parsing", () => {
    it("can be created from an RFC 9557 string", () => {
      const zdt = Temporal.ZonedDateTime.from(
        "2021-08-19T17:01:00+01:00[Europe/London]"
      );
      expect(zdt.toString()).toContain(
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
  });

  describe("arithmetic methods", () => {
    const z1 = Temporal.ZonedDateTime.from("2020-01-01T12:00:00+00:00[UTC]");
    const z2 = Temporal.ZonedDateTime.from("2020-01-02T12:00:00+00:00[UTC]");

    it("add() advances the date/time", () => {
      const later = z1.add({ days: 1 });
      expect(later.day).toBe(2);
    });

    it("compare() method works for two ZonedDateTime objects", () => {
      expect(Temporal.ZonedDateTime.compare(z1, z2)).toBe(-1);
      expect(Temporal.ZonedDateTime.compare(z2, z1)).toBe(1);
      expect(Temporal.ZonedDateTime.compare(z1, z1)).toBe(0);
    });

    it("equals() verifies if two ZonedDateTime are equal", () => {
      const same = Temporal.ZonedDateTime.from("2020-01-01T12:00:00[UTC]");
      expect(z1.equals(same)).toBe(true);
    });

    it("since() returns correct duration", () => {
      const d = z2.since(z1);

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
      const earlier = z1.subtract({ hours: 1 });
      expect(earlier.hour).toBeLessThan(z1.hour);
    });

    it("toInstant() returns a Temporal.Instant", () => {
      const inst = z1.toInstant();
      expect(inst).toBeInstanceOf(Temporal.Instant);
    });

    it("toString() returns an ISO string representation", () => {
      const str = z1.toString();
      expect(typeof str).toBe("string");
      expect(str).toBe("2020-01-01T12:00:00+00:00[UTC]");
    });

    it("until() returns correct duration", () => {
      const d = z1.until(z2);

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
        +z1;
      }).toThrow();
    });

    it("with() method returns a new ZonedDateTime with updated fields", () => {
      const updated = z1.with({ day: 15 });
      expect(updated.day).toBe(15);
      expect(updated.month).toBe(z1.month);
    });

    it("withTimeZone() method returns a new ZonedDateTime with replaceing timezone", () => {
      const updated = z1.withTimeZone("Asia/Tokyo");
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
      // expect(typeof zdt.epochNanoseconds).toBe("BigInt");
      expect(typeof zdt.epochNanoseconds).toBe("number");
      // expect(zdt.epochNanoseconds).toBeGreaterThan(0n);
      expect(zdt.epochNanoseconds).toBeGreaterThan(0);
    });

    it("has correct toStringTag", () => {
      expect(Object.prototype.toString.call(zdt)).toBe(
        "[object Temporal.ZonedDateTime]"
      );
    });
  });
});
