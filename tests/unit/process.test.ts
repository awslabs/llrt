import defaultImport from "process";
import * as namedImport from "process";

describe("process", () => {
  it("should have a process env", () => {
    expect(defaultImport.env).toEqual(process.env);
    expect(namedImport.env).toEqual(process.env);
  });

  it("should have a process cwd", () => {
    expect(defaultImport.cwd()).toEqual(process.cwd());
    expect(namedImport.cwd()).toEqual(process.cwd());
  });

  it("should have a process argv0", () => {
    expect(defaultImport.argv0).toEqual(process.argv0);
    expect(namedImport.argv0).toEqual(process.argv0);
  });

  it("should have a process argv", () => {
    expect(defaultImport.argv).toEqual(process.argv);
    expect(namedImport.argv).toEqual(process.argv);
  });

  it("should have a process platform", () => {
    expect(defaultImport.platform).toEqual(process.platform);
    expect(namedImport.platform).toEqual(process.platform);
  });

  it("should have a process arch", () => {
    expect(defaultImport.arch).toEqual(process.arch);
    expect(namedImport.arch).toEqual(process.arch);
  });

  it("should have a process hrtime", () => {
    expect(defaultImport.hrtime.bigint() > 0).toBeTruthy();
    expect(namedImport.hrtime.bigint() > 0).toBeTruthy();
  });

  it("should have a process release", () => {
    expect(defaultImport.release).toEqual(process.release);
    expect(namedImport.release).toEqual(process.release);
  });

  it("should have a process version", () => {
    expect(defaultImport.version).toEqual(process.version);
    expect(namedImport.version).toEqual(process.version);
  });

  it("should have a process versions", () => {
    expect(defaultImport.versions).toEqual(process.versions);
    expect(namedImport.versions).toEqual(process.versions);
  });

  it("should have a process exit", () => {
    expect(defaultImport.exit).toEqual(process.exit);
    expect(namedImport.exit).toEqual(process.exit);
  });
});
