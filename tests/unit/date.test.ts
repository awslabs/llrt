describe("Date.prototype.toLocaleString with timezone", () => {
  it("should support timeZone option", () => {
    const date = new Date("2022-03-02T15:45:34Z");

    const denver = date.toLocaleString("en-US", { timeZone: "America/Denver" });
    const tokyo = date.toLocaleString("en-US", { timeZone: "Asia/Tokyo" });

    // Denver should show 8:45 (UTC-7 in March)
    expect(denver).toContain("8:45");
    // Tokyo should show March 3 (next day) 00:45 (UTC+9)
    // CLDR en-US short date format is M/d/yy (no leading zeros)
    expect(tokyo).toContain("3/3");
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
