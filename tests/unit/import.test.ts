const CWD = process.cwd();

it("should import a js file", async () => {
  const mod = await import(`${CWD}/fixtures/hello.js`)

  expect(mod.hello).toEqual("hello world!");
});

it("should import a json file", async () => {
  const mod = await import(`${CWD}/package.json`)

  expect(mod.default.private).toEqual(true);
});
