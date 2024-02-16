import path from "path";

describe("path.basename", () => {
  it("should return the last portion of a path", () => {
    assert.strictEqual(path.basename("/foo/bar/baz.txt"), "baz.txt");
    assert.strictEqual(path.basename("/foo/bar/baz.txt", ".txt"), "baz");
    assert.strictEqual(path.basename("/foo/bar/baz/"), "baz");
    assert.strictEqual(path.basename("/foo/bar/baz"), "baz");
    assert.strictEqual(path.basename("baz.txt"), "baz.txt");
    assert.strictEqual(path.basename("/foo/bar/"), "bar");
    assert.strictEqual(path.basename("/foo/bar"), "bar");
    assert.strictEqual(path.basename("/foo/"), "foo");
    assert.strictEqual(path.basename("/foo"), "foo");
    assert.strictEqual(path.basename("/"), "/");
    assert.strictEqual(path.basename(""), ".");
  });
});

describe("path.dirname", () => {
  it("should return the directory path of a given path", () => {
    assert.strictEqual(path.dirname("/foo/bar/baz.txt"), "/foo/bar");
    assert.strictEqual(path.dirname("/foo/bar/baz/"), "/foo/bar");
    assert.strictEqual(path.dirname("/foo/bar/baz"), "/foo/bar");
    assert.strictEqual(path.dirname("/foo/bar/"), "/foo");
    assert.strictEqual(path.dirname("/foo/bar"), "/foo");
    assert.strictEqual(path.dirname("/foo/"), "/", "3");
    assert.strictEqual(path.dirname("/foo"), "/", "4");
    assert.strictEqual(path.dirname("/"), "/", "5");
    assert.strictEqual(path.dirname("baz.txt"), ".", "6");
    assert.strictEqual(path.dirname(""), ".", "7");
  });
});

describe("path.extname", () => {
  it("should return the extension of a given path", () => {
    assert.strictEqual(path.extname("/foo/bar/baz.txt"), ".txt");
    assert.strictEqual(path.extname("/foo/bar/baz.tar.gz"), ".gz");
    assert.strictEqual(path.extname("/foo/bar/baz."), ".");
    assert.strictEqual(path.extname("/foo/bar/baz"), "");
    assert.strictEqual(path.extname("baz.txt"), ".txt");
    assert.strictEqual(path.extname("baz.tar.gz"), ".gz");
    assert.strictEqual(path.extname("baz."), ".");
    assert.strictEqual(path.extname("baz"), "");
    assert.strictEqual(path.extname(".baz"), "");
    assert.strictEqual(path.extname(""), "");
  });
});

describe("path.format", () => {
  it("should return a path string from an object", () => {
    const pathObj1 = {
      root: "/",
      dir: "/foo/bar",
      base: "baz.txt",
      ext: ".txt",
      name: "baz",
    };
    const pathObj2 = {
      dir: "/foo/bar",
      base: "baz.txt",
    };
    const pathObj3 = {
      root: "/",
      base: "baz.txt",
    };
    const pathObj4 = {
      name: "baz",
    };
    assert.strictEqual(path.format(pathObj1), "/foo/bar/baz.txt");
    assert.strictEqual(path.format(pathObj2), "/foo/bar/baz.txt");
    assert.strictEqual(path.format(pathObj3), "/baz.txt");
    assert.strictEqual(path.format(pathObj4), "baz");
  });
});

describe("path.parse", () => {
  it("should return an object from a path string", () => {
    const pathStr1 = "/foo/bar/baz.txt";
    const pathStr2 = "/foo/bar/baz/";
    const pathStr3 = "baz.txt";
    const pathStr4 = "/foo/bar/baz.tar.gz";

    const pathObj1 = {
      root: "/",
      dir: "/foo/bar",
      base: "baz.txt",
      ext: ".txt",
      name: "baz",
    };
    const pathObj2 = {
      root: "/",
      dir: "/foo/bar",
      base: "baz",
      ext: "",
      name: "baz",
    };
    const pathObj3 = {
      root: "",
      dir: "",
      base: "baz.txt",
      ext: ".txt",
      name: "baz",
    };
    const pathObj4 = {
      root: "/",
      dir: "/foo/bar",
      base: "baz.tar.gz",
      ext: ".gz",
      name: "baz.tar",
    };
    assert.deepStrictEqual(path.parse(pathStr1), pathObj1);
    assert.deepStrictEqual(path.parse(pathStr2), pathObj2);
    assert.deepStrictEqual(path.parse(pathStr3), pathObj3);
    assert.deepStrictEqual(path.parse(pathStr4), pathObj4);
  });
});

describe("path.join", () => {
  it("should concatenate path segments and normalize the resulting path", () => {
    assert.strictEqual(
      path.join("/foo", "bar", "baz/asdf", "quux", ".."),
      "/foo/bar/baz/asdf"
    );
    assert.strictEqual(
      path.join("/foo", "bar", "baz", "/asdf", "quux"),
      "/foo/bar/baz/asdf/quux"
    );
    assert.strictEqual(
      path.join("/", "foo", "bar", "baz", "../asdf", "quux"),
      "/foo/bar/asdf/quux"
    );
  });
});

describe("path.resolve", () => {
  it("should resolve a sequence of paths and return an absolute path", () => {
    assert.strictEqual(path.resolve("/foo/bar", "./baz"), "/foo/bar/baz");
    assert.strictEqual(path.resolve("/foo/bar", "/tmp/file/"), "/tmp/file");
    assert.strictEqual(
      path.resolve("wwwroot", "static_files/png/", "../gif/image.gif"),
      path.join(process.cwd(), "wwwroot", "static_files", "gif", "image.gif")
    );
  });
});

describe("path.normalize", () => {
  it("should normalize a path string", () => {
    assert.strictEqual(
      path.normalize("/foo/bar//baz/asdf/quux/.."),
      "/foo/bar/baz/asdf"
    );
    assert.strictEqual(
      path.normalize("foo/bar//baz/asdf/quux/.."),
      "foo/bar/baz/asdf"
    );
    assert.strictEqual(path.normalize("foo/bar/../baz/asdf"), "foo/baz/asdf");
  });
});

describe("path.isAbsolute", () => {
  it("should determine if a path is absolute", () => {
    assert.strictEqual(path.isAbsolute("/foo/bar/baz"), true);
    assert.strictEqual(path.isAbsolute("////foo/bar/baz"), true);
    assert.strictEqual(path.isAbsolute("foo/bar/baz"), false);
    assert.strictEqual(path.isAbsolute("/"), true);
    assert.strictEqual(path.isAbsolute("."), false);
  });
});
