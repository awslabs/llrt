const CWD = process.cwd();

describe("import", () => {
  it("should import a js file (absolute path)", async () => {
    const mod = await import(`${CWD}/fixtures/hello.js`);

    expect(mod.hello).toEqual("hello world!");
  });

  it("should import a json file (absolute path)", async () => {
    async function importThatAvoidsTranspilation(path: string) {
      const fn = new Function(
        "path",
        'return import(path, {with: { type: "json" }});'
      );
      return fn(path);
    }

    const mod = await importThatAvoidsTranspilation(`${CWD}/package.json`);

    expect(mod.default.private).toEqual(true);
  });

  it("should import a js file (relative path)", async () => {
    const mod = await import("../../fixtures/hello.js");

    expect(mod.hello).toEqual("hello world!");
  });

  it("should import a json file (relative path)", async () => {
    const mod = await import("../../fixtures/package.json", {
      with: { type: "json" },
    });

    expect(mod.default.private).toEqual(true);
  });

  it("should import a json file (path unspecified)", async () => {
    const mod = await import("package.json", { with: { type: "json" } });

    expect(mod.default.private).toEqual(true);
  });

  it("should have import.meta.url", async () => {
    const url = import.meta.url;
    // Verify import.meta.url structure without depending on CWD
    // (test can be run from any directory)
    expect(url.startsWith("file://")).toEqual(true);
    // Normalize path separators for cross-platform compatibility
    const normalizedUrl = url.replaceAll("\\", "/");
    expect(
      normalizedUrl.endsWith("/bundle/js/__tests__/unit/import.test.js")
    ).toEqual(true);
  });

  it("regression testing for issue #1492", async () => {
    await import(`${CWD}/fixtures/test1492/index.js`);
  });
});
