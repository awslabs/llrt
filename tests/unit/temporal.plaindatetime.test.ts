describe("Temporal.PlainDateTime", () => {
  describe("creation and parsing", () => {
    it("can be created from an RFC 9557 string", () => {
      const pdt = Temporal.PlainDateTime.from("2021-08-19T17:01:00");
      expect(pdt.toString()).toContain("2021-08-19T17:01:00");
    });

    it("can be created from an own object", () => {
      const pdt1 = Temporal.PlainDateTime.from("2021-08-19T17:01:00");
      const pdt2 = Temporal.PlainDateTime.from(pdt1);
      expect(pdt1).toEqual(pdt2);
    });

    it("can be created from an object containing properties", () => {
      let pdt = Temporal.PlainDateTime.from({
        year: 1995,
        month: 12,
        day: 7,
        hour: 15,
      });

      expect(pdt.toString()).toEqual("1995-12-07T15:00:00");
    });
  });

  describe("arithmetic methods", () => {
    const pdt1 = Temporal.PlainDateTime.from("2020-01-01T12:00:00");
    const pdt2 = Temporal.PlainDateTime.from("2020-01-02T12:00:00");

    it("add() advances the date/time", () => {
      const later = pdt1.add({ days: 1 });
      expect(later.day).toBe(2);
    });

    it("compare() method works for two PlainDateTime objects", () => {
      expect(Temporal.PlainDateTime.compare(pdt1, pdt2)).toBe(-1);
      expect(Temporal.PlainDateTime.compare(pdt2, pdt1)).toBe(1);
      expect(Temporal.PlainDateTime.compare(pdt1, pdt1)).toBe(0);
    });

    it("equals() verifies if two PlainDateTime are equal", () => {
      const same = Temporal.PlainDateTime.from("2020-01-01T12:00:00");
      expect(pdt1.equals(same)).toBe(true);
    });

    it("round() supports various forms, roundingMode, and roundingIncrement", () => {
      const zFrac = pdt1.add({ milliseconds: 500 });
      const rounded = zFrac.round({ smallestUnit: "second" });
      expect(rounded.second).toBe(1);

      const truncated = zFrac.round({
        smallestUnit: "second",
        roundingMode: "trunc",
      });
      expect(truncated.second).toBe(0);

      const zMulti = pdt1.add({ minutes: 7, seconds: 15 });
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
      const d = pdt2.since(pdt1);

      expect(d.sign).toBe(1);
      expect(d.years).toBe(0);
      expect(d.months).toBe(0);
      expect(d.days).toBe(1);
      expect(d.hours).toBe(0);
      expect(d.minutes).toBe(0);
      expect(d.seconds).toBe(0);
      expect(d.milliseconds).toBe(0);
      expect(d.microseconds).toBe(0);
      expect(d.nanoseconds).toBe(0);
    });

    it("subtract() decrements by a duration", () => {
      const earlier = pdt1.subtract({ hours: 1 });
      expect(earlier.hour).toBeLessThan(pdt1.hour);
    });

    it("toPlainDate() returns a Temporal.PlainDate", () => {
      const pd1 = pdt1.toPlainDate();
      expect(pd1).toBeInstanceOf(Temporal.PlainDate);
      expect(pd1.toString()).toBe("2020-01-01");
    });

    it("toPlainTime() returns a Temporal.PlainTime", () => {
      const pt1 = pdt1.toPlainTime();
      expect(pt1).toBeInstanceOf(Temporal.PlainTime);
      expect(pt1.toString()).toBe("12:00:00");
    });

    it("toString() returns an ISO string representation", () => {
      const str = pdt1.toString();
      expect(typeof str).toBe("string");
      expect(str).toBe("2020-01-01T12:00:00");
    });

    it("until() returns correct duration", () => {
      const d = pdt1.until(pdt2);

      expect(d.sign).toBe(1);
      expect(d.years).toBe(0);
      expect(d.months).toBe(0);
      expect(d.days).toBe(1);
      expect(d.hours).toBe(0);
      expect(d.minutes).toBe(0);
      expect(d.seconds).toBe(0);
      expect(d.milliseconds).toBe(0);
      expect(d.microseconds).toBe(0);
      expect(d.nanoseconds).toBe(0);
    });

    it("valueOf() throws a TypeError", () => {
      expect(() => {
        // eslint-disable-next-line @typescript-eslint/no-unused-expressions
        +pdt1;
      }).toThrow();
    });

    it("with() method returns a new PlainDateTime with updated fields", () => {
      const updated = pdt1.with({ day: 15 });
      expect(updated.day).toBe(15);
      expect(updated.month).toBe(pdt1.month);
    });
  });

  describe("properties", () => {
    const pdt = Temporal.PlainDateTime.from("2020-01-01T12:00:00");

    it("should have correct date and time properties", () => {
      expect(pdt.year).toBe(2020);
      expect(pdt.month).toBe(1);
      expect(pdt.day).toBe(1);
      expect(pdt.hour).toBe(12);
      expect(pdt.minute).toBe(0);
      expect(pdt.second).toBe(0);
      expect(pdt.millisecond).toBe(0);
      expect(pdt.microsecond).toBe(0);
      expect(pdt.nanosecond).toBe(0);

      expect(pdt.dayOfYear).toBe(1);
      expect(pdt.daysInMonth).toBe(31);
      expect(pdt.daysInYear).toBe(366);
      expect(pdt.inLeapYear).toBe(true);
    });

    it("has correct toStringTag", () => {
      expect(Object.prototype.toString.call(pdt)).toBe(
        "[object Temporal.PlainDateTime]"
      );
    });
  });
});
