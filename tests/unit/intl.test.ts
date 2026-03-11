describe("Intl.supportedValuesOf", () => {
  it("should return an array", () => {
    const zones = Intl.supportedValuesOf("timeZone");
    expect(Array.isArray(zones)).toBe(true);
  });

  it("should contain common timezones", () => {
    const zones = Intl.supportedValuesOf("timeZone");
    expect(zones).toContain("America/New_York");
    expect(zones).toContain("America/Los_Angeles");
    expect(zones).toContain("America/Denver");
    expect(zones).toContain("Europe/London");
    expect(zones).toContain("Europe/Paris");
    expect(zones).toContain("Asia/Tokyo");
    expect(zones).toContain("UTC");
  });

  it("should return many timezones", () => {
    const zones = Intl.supportedValuesOf("timeZone");
    expect(zones.length).toBeGreaterThan(400);
  });
});

describe("Intl.DateTimeFormat", () => {
  describe("basic functionality", () => {
    it("should be defined globally", () => {
      expect(typeof Intl).toBe("object");
      expect(typeof Intl.DateTimeFormat).toBe("function");
    });

    it("should have correct Symbol.toStringTag", () => {
      const formatter = new Intl.DateTimeFormat("en-US");
      expect(formatter[Symbol.toStringTag]).toBe("Intl.DateTimeFormat");
    });
  });

  describe("formatToParts", () => {
    it("should return an array of parts", () => {
      const formatter = new Intl.DateTimeFormat("en-US", {
        timeZone: "UTC",
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
      });
      const date = new Date("2022-03-02T15:45:34Z");
      const parts = formatter.formatToParts(date);

      expect(Array.isArray(parts)).toBe(true);
      expect(parts.length).toBeGreaterThan(0);
    });

    it("should format date in specified timezone", () => {
      const formatter = new Intl.DateTimeFormat("en-US", {
        timeZone: "America/Denver",
        hour12: false,
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
      // March 2, 2022 15:45:34 UTC = March 2, 2022 08:45:34 Denver (MST -7)
      const date = new Date("2022-03-02T15:45:34Z");
      const parts = formatter.formatToParts(date);

      const hourPart = parts.find((p: any) => p.type === "hour");
      expect(hourPart?.value).toBe("08");
    });

    it("should handle DST correctly", () => {
      const formatter = new Intl.DateTimeFormat("en-US", {
        timeZone: "America/Denver",
        hour12: false,
        hour: "2-digit",
      });

      // June 15, 2024 - DST is in effect (MDT -6)
      const summerDate = new Date("2024-06-15T18:00:00Z");
      const summerParts = formatter.formatToParts(summerDate);
      const summerHour = summerParts.find((p: any) => p.type === "hour");
      expect(summerHour?.value).toBe("12"); // 18 UTC - 6 = 12

      // January 15, 2024 - No DST (MST -7)
      const winterDate = new Date("2024-01-15T18:00:00Z");
      const winterParts = formatter.formatToParts(winterDate);
      const winterHour = winterParts.find((p: any) => p.type === "hour");
      expect(winterHour?.value).toBe("11"); // 18 UTC - 7 = 11
    });
  });

  describe("format", () => {
    it("should return a formatted string", () => {
      const formatter = new Intl.DateTimeFormat("en-US", {
        timeZone: "UTC",
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
      });
      const date = new Date("2022-03-02T15:45:34Z");
      const result = formatter.format(date);

      expect(typeof result).toBe("string");
      expect(result).toContain("03");
      expect(result).toContain("02");
      expect(result).toContain("2022");
    });
  });

  describe("resolvedOptions", () => {
    it("should return resolved options", () => {
      const formatter = new Intl.DateTimeFormat("en-US", {
        timeZone: "America/Denver",
        year: "numeric",
        month: "2-digit",
      });
      const options = formatter.resolvedOptions();

      expect(options.timeZone).toBe("America/Denver");
      expect(options.locale).toBe("en-US");
      expect(options.year).toBe("numeric");
      expect(options.month).toBe("2-digit");
    });

    it("should default to system timezone when not specified", () => {
      const formatter = new Intl.DateTimeFormat("en-US");
      const options = formatter.resolvedOptions();

      expect(typeof options.timeZone).toBe("string");
      expect(options.timeZone.length).toBeGreaterThan(0);
    });
  });
});
