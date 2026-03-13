describe("Temporal.Now", () => {
  describe("arithmetic methods", () => {
    it("instant() returns a Temporal.Instant", () => {
      const inst = Temporal.Now.instant();
      expect(inst).toBeInstanceOf(Temporal.Instant);
    });

    it("plainDateISO() returns a Temporal.PlainDate", () => {
      const obj = Temporal.Now.plainDateISO();
      expect(obj).toBeInstanceOf(Temporal.PlainDate);
    });

    it("plainDateTimeISO() returns a Temporal.PlainDateTime", () => {
      const obj = Temporal.Now.plainDateTimeISO();
      expect(obj).toBeInstanceOf(Temporal.PlainDateTime);
    });

    it("plainTimeISO() returns a Temporal.PlainTime", () => {
      const obj = Temporal.Now.plainTimeISO();
      expect(obj).toBeInstanceOf(Temporal.PlainTime);
    });

    it("zonedDateTimeISO() returns a Temporal.ZonedDateTime", () => {
      const obj = Temporal.Now.zonedDateTimeISO();
      expect(obj).toBeInstanceOf(Temporal.ZonedDateTime);
    });
  });

  describe("properties", () => {
    it("has correct toStringTag", () => {
      expect(Object.prototype.toString.call(Temporal.Now)).toBe(
        "[object Temporal.Now]"
      );
    });
  });
});
