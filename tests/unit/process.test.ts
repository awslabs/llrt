import defaultImport from "process";
import * as namedImport from "process";

describe("process", () => {
  it("should have a process env", () => {
    assert.equal(defaultImport.env, process.env);
    assert.equal(namedImport.env, process.env);
  });

  it("should have a process cwd", () => {
    assert.equal(defaultImport.cwd(), process.cwd());
    assert.equal(namedImport.cwd(), process.cwd());
  });

  it("should have a process argv0", () => {
    assert.equal(defaultImport.argv0, process.argv0);
    assert.equal(namedImport.argv0, process.argv0);
  });

  it("should have a process argv", () => {
    assert.deepEqual(defaultImport.argv, process.argv);
    assert.deepEqual(namedImport.argv, process.argv);
  });

  it("should have a process platform", () => {
    assert.equal(defaultImport.platform, process.platform);
    assert.equal(namedImport.platform, process.platform);
  });

  it("should have a process arch", () => {
    assert.equal(defaultImport.arch, process.arch);
    assert.equal(namedImport.arch, process.arch);
  });

  it("should have a process hrtime", () => {
    assert.ok(defaultImport.hrtime.bigint() > 0);
    assert.ok(namedImport.hrtime.bigint() > 0);
  });

  it("should have a process release", () => {
    assert.equal(defaultImport.release, process.release);
    assert.equal(namedImport.release, process.release);
  });

  it("should have a process version", () => {
    assert.equal(defaultImport.version, process.version);
    assert.equal(namedImport.version, process.version);
  });

  it("should have a process versions", () => {
    assert.equal(defaultImport.versions, process.versions);
    assert.equal(namedImport.versions, process.versions);
  });

  it("should have a process exit", () => {
    assert.equal(defaultImport.exit, process.exit);
    assert.equal(namedImport.exit, process.exit);
  });
});
