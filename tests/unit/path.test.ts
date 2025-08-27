import defaultImport from "node:path";
import legacyImport from "path";
import * as legacyNamedImport from "path";

import path from "node:path";
import { platform } from "node:os";
const IS_WINDOWS = platform() === "win32";

const normalizeSeparator = (string: string) => {
  return string.replaceAll("\\", path.sep).replaceAll("/", path.sep);
};

//relative depends on cwd if any argument is relative
const calculateRelativeDepth = (from: string, to: string) => {
  const fromParts = path.resolve(from).split(path.sep);
  const toParts = path.resolve(to).split(path.sep);

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
  return `${`..${path.sep}`.repeat(upLevels)}${downPath}`;
};

const modules = {
  "node:path": defaultImport,
  path: legacyImport,
  "* as path": legacyNamedImport,
};
for (const module in modules) {
  const {
    basename,
    dirname,
    extname,
    format,
    parse,
    join,
    resolve,
    normalize,
    isAbsolute,
    relative,
  } = modules[module];

  describe(module, () => {
    describe("basename", () => {
      it("should return the last portion of a path", () => {
        expect(basename("C:/foo/bar/baz.txt")).toEqual("baz.txt");
        expect(basename("/foo/bar/baz.txt")).toEqual("baz.txt");
        expect(basename("/foo/bar/baz.txt", ".txt")).toEqual("baz");
        expect(basename("/foo/bar/baz/")).toEqual("baz");
        expect(basename("/foo/bar/baz")).toEqual("baz");
        expect(basename("baz.txt")).toEqual("baz.txt");
        expect(basename("/foo/bar/")).toEqual("bar");
        expect(basename("/foo/bar")).toEqual("bar");
        expect(basename("/foo/")).toEqual("foo");
        expect(basename("/foo")).toEqual("foo");
        expect(basename("/")).toEqual("");
        expect(basename("")).toEqual("");
      });
    });

    describe("dirname", () => {
      it("should return the directory path of a given path", () => {
        expect(dirname("/foo/bar/baz.txt")).toEqual("/foo/bar");
        expect(dirname("/foo/bar/baz/")).toEqual("/foo/bar");
        expect(dirname("/foo/bar/baz")).toEqual("/foo/bar");
        expect(dirname("/foo/bar/")).toEqual("/foo");
        expect(dirname("/foo/bar")).toEqual("/foo");
        expect(dirname("/foo/")).toEqual("/");
        expect(dirname("/foo")).toEqual("/");
        expect(dirname("/")).toEqual("/");
        expect(dirname("baz.txt")).toEqual(".");
        expect(dirname("")).toEqual(".");
      });
    });

    describe("extname", () => {
      it("should return the extension of a given path", () => {
        expect(extname("/foo/bar/baz.txt")).toEqual(".txt");
        expect(extname("/foo/bar/baz.tar.gz")).toEqual(".gz");
        expect(extname("/foo/bar/baz.")).toEqual(".");
        expect(extname("/foo/bar/baz")).toEqual("");
        expect(extname("baz.txt")).toEqual(".txt");
        expect(extname("baz.tar.gz")).toEqual(".gz");
        expect(extname("baz.")).toEqual(".");
        expect(extname("baz")).toEqual("");
        expect(extname(".baz")).toEqual("");
        expect(extname("")).toEqual("");
      });
    });

    describe("format", () => {
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
        expect(format(pathObj1)).toEqual(
          normalizeSeparator("/foo/bar/baz.txt")
        );
        expect(format(pathObj2)).toEqual(
          normalizeSeparator("/foo/bar/baz.txt")
        );
        expect(format(pathObj3)).toEqual(normalizeSeparator("/baz.txt"));
        expect(format(pathObj4)).toEqual(normalizeSeparator("baz"));
      });
    });

    describe("parse", () => {
      it("should return an object from a path string", () => {
        const pathStr1 = normalizeSeparator("/foo/bar/baz.txt");
        const pathStr2 = normalizeSeparator("/foo/bar/baz/");
        const pathStr3 = normalizeSeparator("baz.txt");
        const pathStr4 = normalizeSeparator("/foo/bar/baz.tar.gz");

        const pathObj1 = {
          root: path.sep,
          dir: "/foo/bar",
          base: "baz.txt",
          ext: ".txt",
          name: "baz",
        };
        const pathObj2 = {
          root: path.sep,
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
          root: path.sep,
          dir: "/foo/bar",
          base: "baz.tar.gz",
          ext: ".gz",
          name: "baz.tar",
        };
        expect(parse(pathStr1)).toStrictEqual(pathObj1);
        expect(parse(pathStr2)).toStrictEqual(pathObj2);
        expect(parse(pathStr3)).toStrictEqual(pathObj3);
        expect(parse(pathStr4)).toStrictEqual(pathObj4);
      });
    });

    describe("join", () => {
      it("should concatenate path segments and normalize the resulting path", () => {
        expect(join("/foo", "bar", "baz/asdf", "quux", "..")).toEqual(
          normalizeSeparator("/foo/bar/baz/asdf")
        );
        expect(join("/foo", "bar", "baz", "/asdf", "quux")).toEqual(
          normalizeSeparator("/foo/bar/baz/asdf/quux")
        );
        expect(join("/", "foo", "bar", "baz", "../asdf", "quux")).toEqual(
          normalizeSeparator("/foo/bar/asdf/quux")
        );
      });
    });

    describe("resolve", () => {
      it("should resolve a sequence of paths and return an absolute path", () => {
        expect(resolve("/foo/bar", "./baz")).toEqual(
          normalizeSeparator("/foo/bar/baz")
        );
        expect(resolve("/foo/bar", "/tmp/file/")).toEqual(
          normalizeSeparator("/tmp/file")
        );

        expect(
          resolve("wwwroot", "static_files/png/", "../gif/image.gif")
        ).toEqual(
          join(process.cwd(), "wwwroot", "static_files", "gif", "image.gif")
        );
      });
    });

    describe("normalize", () => {
      it("should normalize a path string", () => {
        expect(normalize("/foo/bar//baz/asdf/quux/..")).toEqual(
          normalizeSeparator("/foo/bar/baz/asdf")
        );
        expect(normalize("foo/bar//baz/asdf/quux/..")).toEqual(
          normalizeSeparator("foo/bar/baz/asdf")
        );
        expect(normalize("foo/bar/../baz/asdf")).toEqual(
          normalizeSeparator("foo/baz/asdf")
        );
      });
    });

    describe("isAbsolute", () => {
      it("should determine if a path is absolute", () => {
        expect(isAbsolute(normalizeSeparator("/foo/bar/baz"))).toEqual(true);
        expect(isAbsolute(normalizeSeparator("////foo/bar/baz"))).toEqual(true);
        expect(isAbsolute(normalizeSeparator("foo/bar/baz"))).toEqual(false);
        expect(isAbsolute(normalizeSeparator("/"))).toEqual(true);
        expect(isAbsolute(normalizeSeparator("."))).toEqual(false);
      });
    });

    describe("relative", () => {
      it("should return the relative path between two absolute directories", () => {
        const from = normalizeSeparator("/Users/test/dir1");
        const to = normalizeSeparator("/Users/test/dir2");
        const result = relative(from, to);
        expect(result).toBe(normalizeSeparator("../dir2"));
      });

      it("should return the relative path between nested absolute directories", () => {
        const from = normalizeSeparator("dir1/subdir1");
        const to = normalizeSeparator("dir2/subdir2");
        const result = relative(from, to);
        expect(result).toBe(normalizeSeparator("../../dir2/subdir2"));
      });

      it("should return '.' for the same absolute directory", () => {
        const from = normalizeSeparator("/Users/test/dir1");
        const to = normalizeSeparator("/Users/test/dir1");
        const result = relative(from, to);
        expect(result).toBe("");
      });

      it("should return the relative path between two non-absolute directories", () => {
        const from = normalizeSeparator("dir1/subdir1");
        const to = normalizeSeparator("dir2/subdir2");
        const result = relative(from, to);
        expect(result).toBe(normalizeSeparator("../../dir2/subdir2"));
      });

      it("should return the relative path with one non-absolute directory", () => {
        const from = normalizeSeparator("dir1/subdir1");
        const to = normalizeSeparator("/Users/test/dir2/subdir2");

        // Calculate manually how many "../" are needed based on directory depth
        const expected = calculateRelativeDepth(from, to);
        const result = relative(from, to);

        expect(result).toBe(expected);
      });

      it('should return the relative path when "to" is a non-absolute file', () => {
        const from = normalizeSeparator("dir1");
        const to = normalizeSeparator("dir2/file.txt");

        const expected = calculateRelativeDepth(from, to);
        const result = relative(from, to);

        expect(result).toBe(expected);
      });

      it('should return the relative path when "from" is a non-absolute file', () => {
        const from = normalizeSeparator("dir1/file.txt");
        const to = normalizeSeparator("dir2");

        const expected = calculateRelativeDepth(from, to);
        const result = relative(from, to);

        expect(result).toBe(expected);
      });

      it('should return the relative path when both "from" and "to" are non-absolute files', () => {
        const from = normalizeSeparator("dir1/file1.txt");
        const to = normalizeSeparator("dir2/file2.txt");

        const expected = calculateRelativeDepth(from, to);
        const result = relative(from, to);

        expect(result).toBe(expected);
      });

      it("should return the relative path between non-absolute and absolute paths", () => {
        const from = normalizeSeparator("dir1");
        const to = normalizeSeparator("/Users/test/dir2/file.txt");

        const expected = calculateRelativeDepth(from, to);
        const result = relative(from, to);

        expect(result).toBe(expected);
      });
    });
  });
}
