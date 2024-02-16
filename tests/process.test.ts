import _process from "process";

describe("process", () => {
  it("should have a process env", () => {
    assert.equal(_process.env, process.env);
  });

  it("should have a process cwd", () => {
    assert.equal(_process.cwd(), process.cwd());
  });

  it("should have a process argv0", () => {
    assert.equal(_process.argv0, process.argv0);
  });

  it("should have a process argv", () => {
    assert.deepEqual(_process.argv, process.argv);
  });

  it("should have a process platform", () => {
    assert.equal(_process.platform, process.platform);
  });

  it("should have a process arch", () => {
    assert.equal(_process.arch, process.arch);
  });

  it("should have a process hrtime", () => {
    assert.ok(_process.hrtime.bigint() > 0);
  });

  it("should have a process release", () => {
    assert.equal(_process.release, process.release);
  });

  it("should have a process version", () => {
    assert.equal(_process.version, process.version);
  });

  it("should have a process versions", () => {
    assert.equal(_process.versions, process.versions);
  });

  it("should have a process exit", () => {
    assert.equal(_process.exit, process.exit);
  });
});
