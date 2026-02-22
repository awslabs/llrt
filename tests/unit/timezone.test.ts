import { Timezone } from "llrt:timezone";

describe("Timezone", () => {
  describe("module import", () => {
    it("should be importable from llrt:timezone", () => {
      expect(typeof Timezone).toBe("object");
    });

    it("should have getOffset method", () => {
      expect(typeof Timezone.getOffset).toBe("function");
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
});
