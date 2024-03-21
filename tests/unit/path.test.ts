import path from "path";

describe("path.basename", () => {
  it("should return the last portion of a path", () => {
    expect(path.basename("/foo/bar/baz.txt")).toEqual("baz.txt")
    expect(path.basename("/foo/bar/baz.txt", ".txt")).toEqual("baz")
    expect(path.basename("/foo/bar/baz/")).toEqual("baz")
    expect(path.basename("/foo/bar/baz")).toEqual("baz")
    expect(path.basename("baz.txt")).toEqual("baz.txt")
    expect(path.basename("/foo/bar/")).toEqual("bar")
    expect(path.basename("/foo/bar")).toEqual("bar")
    expect(path.basename("/foo/")).toEqual("foo")
    expect(path.basename("/foo")).toEqual("foo")
    expect(path.basename("/")).toEqual("/")
    expect(path.basename("")).toEqual(".")
  });
});

describe("path.dirname", () => {
  it("should return the directory path of a given path", () => {
    expect(path.dirname("/foo/bar/baz.txt")).toEqual("/foo/bar")
    expect(path.dirname("/foo/bar/baz/")).toEqual("/foo/bar")
    expect(path.dirname("/foo/bar/baz")).toEqual("/foo/bar")
    expect(path.dirname("/foo/bar/")).toEqual("/foo")
    expect(path.dirname("/foo/bar")).toEqual("/foo")
    expect(path.dirname("/foo/")).toEqual("/")
    expect(path.dirname("/foo")).toEqual("/")
    expect(path.dirname("/")).toEqual("/")
    expect(path.dirname("baz.txt")).toEqual(".")
    expect(path.dirname("")).toEqual(".")
  });
});

describe("path.extname", () => {
  it("should return the extension of a given path", () => {
    expect(path.extname("/foo/bar/baz.txt")).toEqual(".txt")
    expect(path.extname("/foo/bar/baz.tar.gz")).toEqual(".gz")
    expect(path.extname("/foo/bar/baz.")).toEqual(".")
    expect(path.extname("/foo/bar/baz")).toEqual("")
    expect(path.extname("baz.txt")).toEqual(".txt")
    expect(path.extname("baz.tar.gz")).toEqual(".gz")
    expect(path.extname("baz.")).toEqual(".")
    expect(path.extname("baz")).toEqual("")
    expect(path.extname(".baz")).toEqual("")
    expect(path.extname("")).toEqual("")
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
    expect(path.format(pathObj1)).toEqual("/foo/bar/baz.txt")
    expect(path.format(pathObj2)).toEqual("/foo/bar/baz.txt")
    expect(path.format(pathObj3)).toEqual("/baz.txt")
    expect(path.format(pathObj4)).toEqual("baz")
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
    expect(path.parse(pathStr1)).toStrictEqual(pathObj1)
    expect(path.parse(pathStr2)).toStrictEqual(pathObj2)
    expect(path.parse(pathStr3)).toStrictEqual(pathObj3)
    expect(path.parse(pathStr4)).toStrictEqual(pathObj4)
  });
});

describe("path.join", () => {
  it("should concatenate path segments and normalize the resulting path", () => {
    expect(path.join("/foo", "bar", "baz/asdf", "quux", "..")).toEqual("/foo/bar/baz/asdf")
    expect(path.join("/foo", "bar", "baz", "/asdf", "quux")).toEqual("/foo/bar/baz/asdf/quux")
    expect(path.join("/", "foo", "bar", "baz", "../asdf", "quux")).toEqual("/foo/bar/asdf/quux")
  });
});

describe("path.resolve", () => {
  it("should resolve a sequence of paths and return an absolute path", () => {
    expect(path.resolve("/foo/bar", "./baz")).toEqual("/foo/bar/baz")
    expect(path.resolve("/foo/bar", "/tmp/file/")).toEqual("/tmp/file")
    
    expect(path.resolve("wwwroot", "static_files/png/", "../gif/image.gif")).toEqual(path.join(process.cwd(), "wwwroot", "static_files", "gif", "image.gif"))
  });
});

describe("path.normalize", () => {
  it("should normalize a path string", () => {
    expect(path.normalize("/foo/bar//baz/asdf/quux/..")).toEqual("/foo/bar/baz/asdf")
    expect(path.normalize("foo/bar//baz/asdf/quux/..")).toEqual("foo/bar/baz/asdf")
    expect(path.normalize("foo/bar/../baz/asdf")).toEqual("foo/baz/asdf")
  });
});

describe("path.isAbsolute", () => {
  it("should determine if a path is absolute", () => {
    expect(path.isAbsolute("/foo/bar/baz")).toEqual(true)
    expect(path.isAbsolute("////foo/bar/baz")).toEqual(true)
    expect(path.isAbsolute("foo/bar/baz")).toEqual(false)
    expect(path.isAbsolute("/")).toEqual(true)
    expect(path.isAbsolute(".")).toEqual(false)
  });
});
