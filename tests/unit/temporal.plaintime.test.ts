describe("Temporal.PlainTime", () => {
  describe("creation and parsing", () => {
    it("can be created from an RFC 9557 string", () => {
      const pt = Temporal.PlainTime.from("17:01:00");
      expect(pt.toString()).toContain("17:01:00");
    });

    it("can be created from an own object", () => {
      const pt1 = Temporal.PlainTime.from("17:01:00");
      const pt2 = Temporal.PlainTime.from(pt1);
      expect(pt1).toEqual(pt2);
    });

    it("can be created from an object containing properties", () => {
      let time = Temporal.PlainTime.from({
        hour: 19,
        minute: 39,
        second: 9,
        millisecond: 68,
        microsecond: 346,
        nanosecond: 205,
      });
      expect(time.toString()).toEqual("19:39:09.068346205");
    });
  });

  describe("arithmetic methods", () => {
    const pt1 = Temporal.PlainTime.from("12:00:00");
    const pt2 = Temporal.PlainTime.from("12:00:01");

    it("add() advances the time", () => {
      const later = pt1.add({ hours: 1 });
      expect(later.hour).toBe(13);
    });

    it("compare() method works for two PlainTime objects", () => {
      expect(Temporal.PlainTime.compare(pt1, pt2)).toBe(-1);
      expect(Temporal.PlainTime.compare(pt2, pt1)).toBe(1);
      expect(Temporal.PlainTime.compare(pt1, pt1)).toBe(0);
    });

    it("equals() verifies if two PlainTime are equal", () => {
      const same = Temporal.PlainTime.from("12:00:00");
      expect(pt1.equals(same)).toBe(true);
    });

    it("round() supports various forms, roundingMode, and roundingIncrement", () => {
      const zFrac = pt1.add({ milliseconds: 500 });
      const rounded = zFrac.round({ smallestUnit: "second" });
      expect(rounded.second).toBe(1);

      const truncated = zFrac.round({
        smallestUnit: "second",
        roundingMode: "trunc",
      });
      expect(truncated.second).toBe(0);

      const zMulti = pt1.add({ minutes: 7, seconds: 15 });
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
      const d = pt2.since(pt1);

      expect(d.sign).toBe(1);
      expect(d.hours).toBe(0);
      expect(d.minutes).toBe(0);
      expect(d.seconds).toBe(1);
      expect(d.milliseconds).toBe(0);
      expect(d.microseconds).toBe(0);
      expect(d.nanoseconds).toBe(0);
    });

    it("subtract() decrements by a duration", () => {
      const earlier = pt1.subtract({ hours: 1 });
      expect(earlier.hour).toBeLessThan(pt1.hour);
    });

    it("toString() returns an ISO string representation", () => {
      const str = pt1.toString();
      expect(typeof str).toBe("string");
      expect(str).toBe("12:00:00");
    });

    it("until() returns correct duration", () => {
      const d = pt1.until(pt2);

      expect(d.sign).toBe(1);
      expect(d.hours).toBe(0);
      expect(d.minutes).toBe(0);
      expect(d.seconds).toBe(1);
      expect(d.milliseconds).toBe(0);
      expect(d.microseconds).toBe(0);
      expect(d.nanoseconds).toBe(0);
    });

    it("valueOf() throws a TypeError", () => {
      expect(() => {
        // eslint-disable-next-line @typescript-eslint/no-unused-expressions
        +pt1;
      }).toThrow();
    });

    it("with() method returns a new PlainTime with updated fields", () => {
      const updated = pt1.with({ hour: 15 });
      expect(updated.hour).toBe(15);
    });
  });

  describe("properties", () => {
    const pt = Temporal.PlainTime.from("12:00:00");

    it("should have correct date and time properties", () => {
      expect(pt.hour).toBe(12);
      expect(pt.minute).toBe(0);
      expect(pt.second).toBe(0);
      expect(pt.millisecond).toBe(0);
      expect(pt.microsecond).toBe(0);
      expect(pt.nanosecond).toBe(0);
    });

    it("has correct toStringTag", () => {
      expect(Object.prototype.toString.call(pt)).toBe(
        "[object Temporal.PlainTime]"
      );
    });
  });
});
