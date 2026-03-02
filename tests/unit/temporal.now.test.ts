describe("Temporal.Now", () => {
  describe("arithmetic methods", () => {
    it("instant() returns a Temporal.Instant", () => {
      const inst = Temporal.Now.instant();
      expect(inst).toBeInstanceOf(Temporal.Instant);
      expect(typeof inst.epochMilliseconds).toBe("number");
    });

    it("zonedDateTimeISO() returns a Temporal.ZonedDateTime", () => {
      const zdt = Temporal.Now.zonedDateTimeISO();
      expect(zdt).toBeInstanceOf(Temporal.ZonedDateTime);
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
