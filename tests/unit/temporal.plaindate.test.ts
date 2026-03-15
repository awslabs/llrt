describe("Temporal.PlainDate", () => {
  describe("creation and parsing", () => {
    it("can be created from an RFC 9557 string", () => {
      const pd1 = Temporal.PlainDate.from("2021-08-19");
      expect(pd1.toString()).toContain("2021-08-19");
    });

    it("can be created from an own object", () => {
      const pd1 = Temporal.PlainDate.from("2021-08-19");
      const pd2 = Temporal.PlainDate.from(pd1);
      expect(pd1).toEqual(pd2);
    });

    it("can be created from an object containing properties", () => {
      const pd1 = Temporal.PlainDate.from({ year: 2006, month: 8, day: 24 });
      expect(pd1.toString()).toEqual("2006-08-24");
    });
  });

  describe("arithmetic methods", () => {
    const pd1 = Temporal.PlainDate.from("2020-01-01");
    const pd2 = Temporal.PlainDate.from("2020-01-02");

    it("add() advances the date", () => {
      const later = pd1.add({ days: 1 });
      expect(later.day).toBe(2);
    });

    it("compare() method works for two PlainDate objects", () => {
      expect(Temporal.PlainDate.compare(pd1, pd2)).toBe(-1);
      expect(Temporal.PlainDate.compare(pd2, pd1)).toBe(1);
      expect(Temporal.PlainDate.compare(pd1, pd1)).toBe(0);
    });

    it("equals() verifies if two PlainDate are equal", () => {
      const same = Temporal.PlainDate.from("2020-01-01");
      expect(pd1.equals(same)).toBe(true);
    });

    it("since() returns correct duration", () => {
      const d = pd2.since(pd1);

      expect(d.sign).toBe(1);
      expect(d.years).toBe(0);
      expect(d.months).toBe(0);
      expect(d.days).toBe(1);
    });

    it("subtract() decrements by a duration", () => {
      const earlier = pd2.subtract({ days: 1 });
      expect(earlier.day).toBeLessThan(pd2.day);
    });

    it("toPlainDateTime() returns a Temporal.PlainDateTime", () => {
      const pdate1 = pd1.toPlainDateTime();
      expect(pdate1).toBeInstanceOf(Temporal.PlainDateTime);
      expect(pdate1.toString()).toBe("2020-01-01T00:00:00");

      const pdate2 = pd1.toPlainDateTime("12:34:56");
      expect(pdate2.toString()).toBe("2020-01-01T12:34:56");

      const pt = Temporal.PlainTime.from("12:34:56");
      const pd3 = pd1.toPlainDateTime(pt);
      expect(pd3.toString()).toBe("2020-01-01T12:34:56");
    });

    it("toZonedDateTime() returns a Temporal.ZonedDateTime", () => {
      const pdt1 = pd1.toZonedDateTime("Asia/Tokyo");
      expect(pdt1).toBeInstanceOf(Temporal.ZonedDateTime);
      expect(pdt1.toString()).toBe("2020-01-01T00:00:00+09:00[Asia/Tokyo]");

      const pdt2 = pd1.toZonedDateTime({ timeZone: "Asia/Tokyo" });
      expect(pdt2.toString()).toBe("2020-01-01T00:00:00+09:00[Asia/Tokyo]");

      const pdt3 = pd1.toZonedDateTime({
        timeZone: "Asia/Tokyo",
        plainTime: "12:34:56",
      });
      expect(pdt3.toString()).toBe("2020-01-01T12:34:56+09:00[Asia/Tokyo]");

      const pt = Temporal.PlainTime.from("12:34:56");
      const pdt4 = pd1.toZonedDateTime({
        timeZone: "Asia/Tokyo",
        plainTime: pt,
      });
      expect(pdt4.toString()).toBe("2020-01-01T12:34:56+09:00[Asia/Tokyo]");
    });

    it("toString() returns an ISO string representation", () => {
      const str = pd1.toString();
      expect(typeof str).toBe("string");
      expect(str).toBe("2020-01-01");
    });

    it("until() returns correct duration", () => {
      const d = pd1.until(pd2);

      expect(d.sign).toBe(1);
      expect(d.years).toBe(0);
      expect(d.months).toBe(0);
      expect(d.days).toBe(1);
    });

    it("valueOf() throws a TypeError", () => {
      expect(() => {
        // eslint-disable-next-line @typescript-eslint/no-unused-expressions
        +pd1;
      }).toThrow();
    });

    it("with() method returns a new PlainDate with updated fields", () => {
      const updated = pd1.with({ day: 15 });
      expect(updated.day).toBe(15);
      expect(updated.month).toBe(pd1.month);
    });
  });

  describe("properties", () => {
    const pdt = Temporal.PlainDate.from("2020-01-01");

    it("should have correct date and time properties", () => {
      expect(pdt.year).toBe(2020);
      expect(pdt.month).toBe(1);
      expect(pdt.day).toBe(1);
      expect(pdt.dayOfYear).toBe(1);
      expect(pdt.daysInMonth).toBe(31);
      expect(pdt.daysInYear).toBe(366);
      expect(pdt.inLeapYear).toBe(true);
    });

    it("has correct toStringTag", () => {
      expect(Object.prototype.toString.call(pdt)).toBe(
        "[object Temporal.PlainDate]"
      );
    });
  });
});
