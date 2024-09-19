// uuid.test.mjs
import {
  v1 as uuidv1,
  v3 as uuidv3,
  v4 as uuidv4,
  v5 as uuidv5,
  v6 as uuidv6,
  v7 as uuidv7,
  v1ToV6 as uuidv1ToV6,
  v6ToV1 as uuidv6ToV1,
  parse,
  stringify,
  validate,
  NIL,
  MAX,
  version,
} from "llrt:uuid";

const UUID_PATTERN =
  /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;

describe("UUID Generation", () => {
  it("should generate a valid v1 UUID", () => {
    const uuid = uuidv1();
    expect(typeof uuid).toEqual("string");
    expect(uuid.length).toEqual(36);
    expect(uuid).toMatch(UUID_PATTERN);
    expect(version(uuid)).toEqual(1);
  });

  it("should generate a valid v3 UUID", () => {
    const uuid = uuidv3("hello", uuidv3.URL);
    expect(typeof uuid).toEqual("string");
    expect(uuid.length).toEqual(36);
    expect(uuid).toMatch(UUID_PATTERN);
    expect(version(uuid)).toEqual(3);
  });

  it("should generate a valid v4 UUID", () => {
    const uuid = uuidv4();
    expect(typeof uuid).toEqual("string");
    expect(uuid.length).toEqual(36);
    expect(uuid).toMatch(UUID_PATTERN);
    expect(version(uuid)).toEqual(4);
  });

  it("should generate a valid v5 UUID", () => {
    const uuid = uuidv5("hello", uuidv5.DNS);
    expect(typeof uuid).toEqual("string");
    expect(uuid.length).toEqual(36);
    expect(uuid).toMatch(UUID_PATTERN);
    expect(version(uuid)).toEqual(5);
  });

  it("should generate a valid v6 UUID", () => {
    const uuid = uuidv6();
    expect(typeof uuid).toEqual("string");
    expect(uuid.length).toEqual(36);
    expect(uuid).toMatch(UUID_PATTERN);
    expect(version(uuid)).toEqual(6);
  });

  it("should generate a valid v7 UUID", () => {
    const uuid = uuidv7();
    expect(typeof uuid).toEqual("string");
    expect(uuid.length).toEqual(36);
    expect(uuid).toMatch(UUID_PATTERN);
    expect(version(uuid)).toEqual(7);
  });

  it("should convert v1 -> v6 and vice versa", () => {
    const v1 = "f4df6856-5238-11ef-a311-d4807f27f0c6";
    const v6 = "1ef5238f-4df6-6856-a311-d4807f27f0c6";

    const convertedv6 = uuidv1ToV6(v1);
    expect(convertedv6).toEqual(v6);
    expect(version(convertedv6)).toEqual(6);

    const convertedv1 = uuidv6ToV1(convertedv6);
    expect(convertedv1).toEqual(v1);
    expect(version(convertedv1)).toEqual(1);
  });

  it("should parse and stringify a UUID", () => {
    const uuid = uuidv1();
    const parsedUuid = parse(uuid);
    const stringifiedUuid = stringify(parsedUuid);
    expect(typeof parsedUuid).toEqual("object");
    expect(typeof stringifiedUuid).toEqual("string");
    expect(stringifiedUuid).toEqual(uuid);
  });

  it("should validate a valid UUID", () => {
    const uuid = uuidv1();
    expect(validate(uuid)).toEqual(true);
  });

  it("should validate an invalid UUID", () => {
    expect(validate("invalid-uuid")).toEqual(false);
  });

  it("should generate a NIL UUID", () => {
    const nilUuid = NIL;
    expect(nilUuid).toEqual("00000000-0000-0000-0000-000000000000");
    expect(version(nilUuid)).toEqual(0);
  });

  it("should generate a MAX UUID", () => {
    const maxUuid = MAX;
    expect(maxUuid).toEqual("ffffffff-ffff-ffff-ffff-ffffffffffff");
    expect(version(maxUuid)).toEqual(15);
  });

  it("should return correct versions", () => {
    const v1 = uuidv1();
    const v3 = uuidv3("hello", uuidv3.URL);
    const v4 = uuidv4();
    const v5 = uuidv5("hello", uuidv3.URL);
    const v6 = uuidv6();
    const v7 = uuidv7();
    expect(version(v1)).toEqual(2);
    expect(version(v3)).toEqual(3);
    expect(version(v4)).toEqual(4);
    expect(version(v5)).toEqual(5);
    expect(version(v6)).toEqual(6);
    expect(version(v7)).toEqual(7);
    expect(version(NIL)).toEqual(0);
    expect(version(MAX)).toEqual(15);
  });
});
