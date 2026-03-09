describe("Temporal.Duration", () => {
  describe("creation and parsing", () => {
    it("can be created from an object", () => {
      const dur = Temporal.Duration.from({ hours: 3, minutes: 30 });
      expect(dur.hours).toBe(3);
      expect(dur.minutes).toBe(30);
      expect(dur.toString()).toBe("PT3H30M");
    });

    it("can be parsed from an ISO 8601 string", () => {
      const dur = Temporal.Duration.from("P1Y2M3DT4H5M6.007008009S");
      expect(dur.years).toBe(1);
      expect(dur.months).toBe(2);
      expect(dur.days).toBe(3);
      expect(dur.hours).toBe(4);
      expect(dur.minutes).toBe(5);
      expect(dur.seconds).toBe(6);
      expect(dur.milliseconds).toBe(7);
      expect(dur.microseconds).toBe(8);
      expect(dur.nanoseconds).toBe(9);
    });

    it("can be created from an own object", () => {
      const dur1 = Temporal.Duration.from("PT0S");
      const dur2 = Temporal.Duration.from(dur1);
      expect(dur1).toEqual(dur2);
    });

    it("has a blank property that detects zero durations", () => {
      const zero = Temporal.Duration.from("PT0S");
      expect(zero.blank).toBe(true);
      const nonzero = Temporal.Duration.from({ seconds: 1 });
      expect(nonzero.blank).toBe(false);
    });
  });

  describe("arithmetic methods", () => {
    const d1 = Temporal.Duration.from({ hours: 1, minutes: 30 });
    const d2 = Temporal.Duration.from({ minutes: 45 });

    it("abs() and negated() returns durations correctly", () => {
      const neg = d1.negated();
      expect(neg.hours).toBe(-1);
      expect(neg.abs().hours).toBe(1);
    });

    it("add() return durations correctly", () => {
      const sum = d1.add(d2);
      expect(sum.hours).toBe(2);
      expect(sum.minutes).toBe(15);
    });

    it("compare() works correctly", () => {
      expect(Temporal.Duration.compare(d1, d2)).toBe(1);
      expect(Temporal.Duration.compare(d2, d1)).toBe(-1);
      expect(Temporal.Duration.compare(d1, d1)).toBe(0);
    });

    it("subtract() return durations correctly", () => {
      const diff = d1.subtract(d2);
      expect(diff.hours).toBe(0);
      expect(diff.minutes).toBe(45);
    });

    it("toJSON() returns the ISO string", () => {
      const dur = Temporal.Duration.from({ days: 40 });
      expect(JSON.stringify(dur)).toBe('"P40D"');
    });

    it("valueOf() throws a TypeError", () => {
      const dur = Temporal.Duration.from({ seconds: 1 });
      expect(() => {
        // eslint-disable-next-line @typescript-eslint/no-unused-expressions
        +dur;
      }).toThrow();
    });

    it("with() returns a new duration with updated fields", () => {
      const dur = Temporal.Duration.from({ hours: 1, minutes: 30 });
      const updated = dur.with({ minutes: 45 });
      expect(updated.hours).toBe(1);
      expect(updated.minutes).toBe(45);
    });
  });

  describe("properties", () => {
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
      const zdt = Temporal.Duration.from("PT0S");
      expect(Object.prototype.toString.call(zdt)).toBe(
        "[object Temporal.Duration]"
      );
    });
  });
});
