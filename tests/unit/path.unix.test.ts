import path from "path";
import { platform } from "os";
const IS_WINDOWS = platform() === "win32";

//path.relative depends on cwd if any argument is relative
function calculateRelativeDepth(from: string, to: string) {
  const fromParts = path.resolve(from).split("/");
  const toParts = path.resolve(to).split("/");

  //find the first index where the paths differ
  let i = 0;
  while (
    i < fromParts.length &&
    i < toParts.length &&
    fromParts[i] === toParts[i]
  ) {
    i++;
  }

  //calculate how many '../' are needed from "from" to reach common base directory
  const upLevels = fromParts.length - i;
  const downPath = toParts.slice(i).join("/");

  //return the correct number of '../' segments followed by the remaining "to" path
  return `${"../".repeat(upLevels)}${downPath}`;
}

if (!IS_WINDOWS) {
  describe("path.basename", () => {
    it("should return the last portion of a path", () => {
      expect(path.basename("/foo/bar/baz.txt")).toEqual("baz.txt");
      expect(path.basename("/foo/bar/baz.txt", ".txt")).toEqual("baz");
      expect(path.basename("/foo/bar/baz/")).toEqual("baz");
      expect(path.basename("/foo/bar/baz")).toEqual("baz");
      expect(path.basename("baz.txt")).toEqual("baz.txt");
      expect(path.basename("/foo/bar/")).toEqual("bar");
      expect(path.basename("/foo/bar")).toEqual("bar");
      expect(path.basename("/foo/")).toEqual("foo");
      expect(path.basename("/foo")).toEqual("foo");
      expect(path.basename("/")).toEqual("");
      expect(path.basename("")).toEqual("");
    });
  });

  describe("path.dirname", () => {
    it("should return the directory path of a given path", () => {
      expect(path.dirname("/foo/bar/baz.txt")).toEqual("/foo/bar");
      expect(path.dirname("/foo/bar/baz/")).toEqual("/foo/bar");
      expect(path.dirname("/foo/bar/baz")).toEqual("/foo/bar");
      expect(path.dirname("/foo/bar/")).toEqual("/foo");
      expect(path.dirname("/foo/bar")).toEqual("/foo");
      expect(path.dirname("/foo/")).toEqual("/");
      expect(path.dirname("/foo")).toEqual("/");
      expect(path.dirname("/")).toEqual("/");
      expect(path.dirname("baz.txt")).toEqual(".");
      expect(path.dirname("")).toEqual(".");
    });
  });

  describe("path.extname", () => {
    it("should return the extension of a given path", () => {
      expect(path.extname("/foo/bar/baz.txt")).toEqual(".txt");
      expect(path.extname("/foo/bar/baz.tar.gz")).toEqual(".gz");
      expect(path.extname("/foo/bar/baz.")).toEqual(".");
      expect(path.extname("/foo/bar/baz")).toEqual("");
      expect(path.extname("baz.txt")).toEqual(".txt");
      expect(path.extname("baz.tar.gz")).toEqual(".gz");
      expect(path.extname("baz.")).toEqual(".");
      expect(path.extname("baz")).toEqual("");
      expect(path.extname(".baz")).toEqual("");
      expect(path.extname("")).toEqual("");
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
      expect(path.format(pathObj1)).toEqual("/foo/bar/baz.txt");
      expect(path.format(pathObj2)).toEqual("/foo/bar/baz.txt");
      expect(path.format(pathObj3)).toEqual("/baz.txt");
      expect(path.format(pathObj4)).toEqual("baz");
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
      expect(path.parse(pathStr1)).toStrictEqual(pathObj1);
      expect(path.parse(pathStr2)).toStrictEqual(pathObj2);
      expect(path.parse(pathStr3)).toStrictEqual(pathObj3);
      expect(path.parse(pathStr4)).toStrictEqual(pathObj4);
    });
  });

  describe("path.join", () => {
    it("should concatenate path segments and normalize the resulting path", () => {
      expect(path.join("/foo", "bar", "baz/asdf", "quux", "..")).toEqual(
        "/foo/bar/baz/asdf"
      );
      expect(path.join("/foo", "bar", "baz", "/asdf", "quux")).toEqual(
        "/foo/bar/baz/asdf/quux"
      );
      expect(path.join("/", "foo", "bar", "baz", "../asdf", "quux")).toEqual(
        "/foo/bar/asdf/quux"
      );
    });
  });

  describe("path.resolve", () => {
    it("should resolve a sequence of paths and return an absolute path", () => {
      expect(path.resolve("/foo/bar", "./baz")).toEqual("/foo/bar/baz");
      expect(path.resolve("/foo/bar", "/tmp/file/")).toEqual("/tmp/file");

      expect(
        path.resolve("wwwroot", "static_files/png/", "../gif/image.gif")
      ).toEqual(
        path.join(process.cwd(), "wwwroot", "static_files", "gif", "image.gif")
      );
    });
  });

  describe("path.normalize", () => {
    it("should normalize a path string", () => {
      expect(path.normalize("/foo/bar//baz/asdf/quux/..")).toEqual(
        "/foo/bar/baz/asdf"
      );
      expect(path.normalize("foo/bar//baz/asdf/quux/..")).toEqual(
        "foo/bar/baz/asdf"
      );
      expect(path.normalize("foo/bar/../baz/asdf")).toEqual("foo/baz/asdf");
    });
  });

  describe("path.isAbsolute", () => {
    it("should determine if a path is absolute", () => {
      expect(path.isAbsolute("/foo/bar/baz")).toEqual(true);
      expect(path.isAbsolute("////foo/bar/baz")).toEqual(true);
      expect(path.isAbsolute("foo/bar/baz")).toEqual(false);
      expect(path.isAbsolute("/")).toEqual(true);
      expect(path.isAbsolute(".")).toEqual(false);
    });
  });

  describe("path.relative", () => {
    it("should return the relative path between two absolute directories", () => {
      const result = path.relative("/Users/test/dir1", "/Users/test/dir2");
      expect(result).toBe("../dir2");
    });

    it("should return the relative path between nested absolute directories", () => {
      const result = path.relative(
        "/Users/test/dir1/subdir1",
        "/Users/test/dir2/subdir2"
      );
      expect(result).toBe("../../dir2/subdir2");
    });

    it("should return '.' for the same absolute directory", () => {
      const result = path.relative("/Users/test/dir1", "/Users/test/dir1");
      expect(result).toBe("");
    });

    it("should return the relative path between two non-absolute directories", () => {
      const result = path.relative("dir1/subdir1", "dir2/subdir2");
      expect(result).toBe("../../dir2/subdir2");
    });

    it("should return the relative path with one non-absolute directory", () => {
      const from = "dir1/subdir1";
      const to = "/Users/test/dir2/subdir2";

      // Calculate manually how many "../" are needed based on directory depth
      const expected = calculateRelativeDepth(from, to);
      const result = path.relative(from, to);

      expect(result).toBe(expected);
    });

    it('should return the relative path when "to" is a non-absolute file', () => {
      const from = "dir1";
      const to = "dir2/file.txt";

      const expected = calculateRelativeDepth(from, to);
      const result = path.relative(from, to);

      expect(result).toBe(expected);
    });

    it('should return the relative path when "from" is a non-absolute file', () => {
      const from = "dir1/file.txt";
      const to = "dir2";

      const expected = calculateRelativeDepth(from, to);
      const result = path.relative(from, to);

      expect(result).toBe(expected);
    });

    it('should return the relative path when both "from" and "to" are non-absolute files', () => {
      const from = "dir1/file1.txt";
      const to = "dir2/file2.txt";

      const expected = calculateRelativeDepth(from, to);
      const result = path.relative(from, to);

      expect(result).toBe(expected);
    });

    it("should return the relative path between non-absolute and absolute paths", () => {
      const from = "dir1";
      const to = "/Users/test/dir2/file.txt";

      const expected = calculateRelativeDepth(from, to);
      const result = path.relative(from, to);

      expect(result).toBe(expected);
    });
  });
}
