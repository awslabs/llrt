// uuid.test.mjs
import {
  v1 as uuidv1,
  v3 as uuidv3,
  v4 as uuidv4,
  v5 as uuidv5,
  parse,
  stringify,
  validate,
  NIL,
  version,
} from "uuid";

const UUID_PATTERN =
  /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;

describe("UUID Generation", () => {
  it("should generate a valid v1 UUID", () => {
    const uuid = uuidv1();
    assert.strictEqual(typeof uuid, "string");
    assert.strictEqual(uuid.length, 36);
    assert(uuid.match(UUID_PATTERN));
    assert.strictEqual(version(uuid), 1);
  });

  it("should generate a valid v3 UUID", () => {
    const uuid = uuidv3("hello", uuidv3.URL);
    assert.strictEqual(typeof uuid, "string");
    assert.strictEqual(uuid.length, 36);
    assert(uuid.match(UUID_PATTERN));
    assert.strictEqual(version(uuid), 3);
  });

  it("should generate a valid v4 UUID", () => {
    const uuid = uuidv4();
    assert.strictEqual(typeof uuid, "string");
    assert.strictEqual(uuid.length, 36);
    assert(uuid.match(UUID_PATTERN));
    assert.strictEqual(version(uuid), 4);
  });

  it("should generate a valid v5 UUID", () => {
    const uuid = uuidv5("hello", uuidv5.DNS);
    assert.strictEqual(typeof uuid, "string");
    assert.strictEqual(uuid.length, 36);
    assert(uuid.match(UUID_PATTERN));
    assert.strictEqual(version(uuid), 5);
  });

  it("should parse and stringify a UUID", () => {
    const uuid = uuidv1();
    const parsedUuid = parse(uuid);
    const stringifiedUuid = stringify(parsedUuid);
    assert.strictEqual(typeof parsedUuid, "object");
    assert.strictEqual(typeof stringifiedUuid, "string");
    assert.strictEqual(stringifiedUuid, uuid);
  });

  it("should validate a valid UUID", () => {
    const uuid = uuidv1();
    assert.strictEqual(validate(uuid), true);
  });

  it("should validate an invalid UUID", () => {
    assert.strictEqual(validate("invalid-uuid"), false);
  });

  it("should generate a NIL UUID", () => {
    const nilUuid = NIL;
    assert.strictEqual(nilUuid, "00000000-0000-0000-0000-000000000000");
    assert.strictEqual(version(nilUuid), 0);
  });

  it("should return correct versions", () => {
    const v1 = uuidv1();
    const v3 = uuidv3("hello", uuidv3.URL);
    const v4 = uuidv4();
    const v5 = uuidv5("hello", uuidv3.URL);
    assert.strictEqual(version(v1), 1);
    assert.strictEqual(version(v3), 3);
    assert.strictEqual(version(v4), 4);
    assert.strictEqual(version(v5), 5);
    assert.strictEqual(version(NIL), 0);
  });
});
