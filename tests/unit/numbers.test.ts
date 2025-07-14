describe("Number to base conversion within 32-bit range", () => {
  const testCases = [
    { num: 0, radix: 2, expected: "0" },
    { num: 1, radix: 2, expected: "1" },
    { num: -1, radix: 2, expected: "-1" },
    { num: NaN, radix: 2, expected: "NaN" },
    { num: Infinity, radix: 2, expected: "Infinity" },

    { num: 255, radix: 16, expected: "ff" },
    { num: -255, radix: 16, expected: "-ff" },
    { num: NaN, radix: 16, expected: "NaN" },
    { num: Infinity, radix: 16, expected: "Infinity" },

    { num: 1023, radix: 8, expected: "1777" },
    { num: -1023, radix: 8, expected: "-1777" },
    { num: NaN, radix: 8, expected: "NaN" },
    { num: Infinity, radix: 8, expected: "Infinity" },

    { num: 123456789, radix: 10, expected: "123456789" },
    { num: -123456789, radix: 10, expected: "-123456789" },
    { num: 12345.6789, radix: 10, expected: "12345.6789" },
    { num: -12345.6789, radix: 10, expected: "-12345.6789" },
    { num: NaN, radix: 10, expected: "NaN" },
    { num: Infinity, radix: 10, expected: "Infinity" },
    { num: -Infinity, radix: 10, expected: "-Infinity" },

    { num: 2147483647, radix: 16, expected: "7fffffff" }, // max 32bit signed
    { num: -2147483648, radix: 16, expected: "-80000000" }, // min 32bit signed
    { num: 12345.6789, radix: 16, expected: "3039.adcc63f14" },
    { num: -12345.6789, radix: 16, expected: "-3039.adcc63f14" },
    { num: NaN, radix: 16, expected: "NaN" },
    { num: Infinity, radix: 16, expected: "Infinity" },
    { num: -Infinity, radix: 16, expected: "-Infinity" },

    { num: 100, radix: 36, expected: "2s" },
    { num: -100, radix: 36, expected: "-2s" },
    { num: NaN, radix: 36, expected: "NaN" },
    { num: Infinity, radix: 36, expected: "Infinity" },
  ];

  testCases.forEach(({ num, radix, expected }) => {
    it(`num=${num} radix=${radix} => ${expected}`, () => {
      const actual = num.toString(radix);
      expect(actual).toBe(expected);
    });
  });
});

describe("Number to base conversion over 32-bit range", () => {
  const num = 1749475325433;

  it("should convert number to binary (base-2)", () => {
    expect(num.toString(2)).toEqual(
      "11001011101010100110110101111010111111001"
    );
  });

  it("should convert number to octal (base-8)", () => {
    expect(num.toString(8)).toEqual("31352466572771");
  });

  it("should convert number to decimal (base-10)", () => {
    expect(num.toString(10)).toEqual("1749475325433");
  });

  it("should convert number to hexadecimal (base-16)", () => {
    expect(num.toString(16)).toEqual("19754daf5f9");
  });

  it("should convert number to base-20", () => {
    expect(num.toString(20)).toEqual("386fb0fdbd");
  });

  it("should convert number to base-26", () => {
    expect(num.toString(26)).toEqual("89l74gagh");
  });

  it("should convert number to base-32", () => {
    expect(num.toString(32)).toEqual("1itadltfp");
  });

  it("should convert number to base-36", () => {
    expect(num.toString(36)).toEqual("mbp4fshl");
  });
});
