const CWD = process.cwd();

describe("import", () => {
  it("should import a js file (absolute path)", async () => {
    const mod = await import(`${CWD}/fixtures/hello.js`);

    expect(mod.hello).toEqual("hello world!");
  });

  it("should import a json file (absolute path)", async () => {
    const mod = await import(`${CWD}/package.json`);

    expect(mod.default.private).toEqual(true);
  });

  it("should import a js file (relative path)", async () => {
    const mod = await import("../../hello.js");

    expect(mod.hello).toEqual("hello world!");
  });

  it("should import a json file (relative path)", async () => {
    const mod = await import("../../package.json");

    expect(mod.default.private).toEqual(true);
  });

  it("should import a js file (path unspecified)", async () => {
    const mod = await import("hello.js");

    expect(mod.hello).toEqual("hello world!");
  });

  it("should import a json file (path unspecified)", async () => {
    const mod = await import("package.json");

    expect(mod.default.private).toEqual(true);
  });

  it("should have import.meta.url", async () => {
    const url = import.meta.url;
    expect(url).toEqual(
      `file://${CWD}/bundle/js/__tests__/unit/import.test.js`.replaceAll(
        "\\",
        "/"
      )
    );
  });
});
