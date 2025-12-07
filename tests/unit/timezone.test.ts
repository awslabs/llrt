describe("Timezone", () => {
  describe("global availability", () => {
    it("should be defined globally", () => {
      expect(typeof Timezone).toBe("object");
    });

    it("should have getOffset method", () => {
      expect(typeof Timezone.getOffset).toBe("function");
    });

    it("should have list method", () => {
      expect(typeof Timezone.list).toBe("function");
    });

    it("should have correct Symbol.toStringTag", () => {
      expect(Timezone[Symbol.toStringTag]).toBe("Timezone");
    });
  });

  describe("getOffset", () => {
    it("should return 0 for UTC", () => {
      const offset = Timezone.getOffset("UTC", Date.now());
      expect(offset).toBe(0);
    });

    it("should return positive offset for Tokyo (UTC+9)", () => {
      const offset = Timezone.getOffset("Asia/Tokyo", Date.now());
      expect(offset).toBe(540); // 9 hours * 60 minutes
    });

    it("should return negative offset for New York", () => {
      const offset = Timezone.getOffset("America/New_York", Date.now());
      // -300 (EST/UTC-5) or -240 (EDT/UTC-4) depending on DST
      expect(offset).toBeLessThan(0);
      expect(offset).toBeGreaterThanOrEqual(-300);
      expect(offset).toBeLessThanOrEqual(-240);
    });

    it("should handle DST transitions correctly", () => {
      // March 10, 2024 2:00 AM is when DST starts in US
      const beforeDst = new Date("2024-03-09T12:00:00Z").getTime();
      const afterDst = new Date("2024-03-11T12:00:00Z").getTime();

      const offsetBefore = Timezone.getOffset("America/Denver", beforeDst);
      const offsetAfter = Timezone.getOffset("America/Denver", afterDst);

      expect(offsetBefore).toBe(-420); // UTC-7 (MST)
      expect(offsetAfter).toBe(-360); // UTC-6 (MDT)
    });

    it("should throw TypeError for invalid timezone", () => {
      expect(() => {
        Timezone.getOffset("Invalid/Timezone", Date.now());
      }).toThrow(TypeError);
    });

    it("should work with epoch 0", () => {
      const offset = Timezone.getOffset("UTC", 0);
      expect(offset).toBe(0);
    });
  });

  describe("list", () => {
    it("should return an array", () => {
      const zones = Timezone.list();
      expect(Array.isArray(zones)).toBe(true);
    });

    it("should contain common timezones", () => {
      const zones = Timezone.list();
      expect(zones).toContain("America/New_York");
      expect(zones).toContain("America/Los_Angeles");
      expect(zones).toContain("America/Denver");
      expect(zones).toContain("Europe/London");
      expect(zones).toContain("Europe/Paris");
      expect(zones).toContain("Asia/Tokyo");
      expect(zones).toContain("UTC");
    });

    it("should return many timezones", () => {
      const zones = Timezone.list();
      expect(zones.length).toBeGreaterThan(400);
    });
  });
});

describe("Timezone module import", () => {
  it("should be importable from llrt:timezone", async () => {
    const { Timezone: TZ } = await import("llrt:timezone");
    expect(typeof TZ).toBe("object");
    expect(typeof TZ.getOffset).toBe("function");
    expect(typeof TZ.list).toBe("function");
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

describe("Date.prototype.toLocaleString with timezone", () => {
  it("should support timeZone option", () => {
    const date = new Date("2022-03-02T15:45:34Z");

    const denver = date.toLocaleString("en-US", { timeZone: "America/Denver" });
    const tokyo = date.toLocaleString("en-US", { timeZone: "Asia/Tokyo" });

    // Denver should show 8:45 (UTC-7 in March)
    expect(denver).toContain("8:45");
    // Tokyo should show March 3 (next day) 00:45 (UTC+9)
    expect(tokyo).toContain("03/03");
  });

  it("should work without timezone option", () => {
    const date = new Date("2022-03-02T15:45:34Z");
    const result = date.toLocaleString();

    expect(typeof result).toBe("string");
    expect(result.length).toBeGreaterThan(0);
  });

  it("should handle Invalid Date", () => {
    const date = new Date("invalid");
    const result = date.toLocaleString("en-US", { timeZone: "UTC" });

    expect(result).toBe("Invalid Date");
  });
});
