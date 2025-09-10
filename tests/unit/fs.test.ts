import defaultImport from "node:fs";
import legacyImport from "fs";
import * as legacyNamedImport from "fs";

import path from "node:path";
import os from "node:os";
const IS_WINDOWS = os.platform() === "win32";

const modules = {
  "node:fs": defaultImport,
  fs: legacyImport,
  "* as fs": legacyNamedImport,
};
for (const module in modules) {
  const {
    constants,
    accessSync,
    readdirSync,
    readFileSync,
    mkdtempSync,
    mkdirSync,
    rmSync,
    rmdirSync,
    statSync,
    writeFileSync,
    promises,
  } = modules[module];

  describe(module, () => {
    describe("readdir", () => {
      it("should read a directory", async () => {
        const dir = await promises.readdir(".cargo");
        expect(dir).toEqual(["config.toml"]);
      });

      it("should read a directory with types", async () => {
        const dir = await promises.readdir(".cargo", { withFileTypes: true });
        expect(dir).toEqual([
          {
            name: "config.toml",
            parentPath: ".cargo",
          },
        ]);
        expect(dir[0].isFile()).toBeTruthy();
      });

      it("should read a directory with types", async () => {
        const dir = await promises.readdir(".cargo/", { withFileTypes: true });
        expect(dir).toEqual([
          {
            name: "config.toml",
            parentPath: ".cargo",
          },
        ]);
        expect(dir[0].isFile()).toBeTruthy();
      });

      it("should read a directory", async () => {
        const dir = await promises.readdir(".cargo");
        expect(dir).toEqual(["config.toml"]);
      });

      it("should read a directory with recursive", async () => {
        const dir = await promises.readdir("fixtures/fs/readdir", {
          recursive: true,
        });
        const compare = (a: string, b: string) => (a >= b ? 1 : -1);
        expect(dir.sort(compare)).toEqual(
          [
            IS_WINDOWS ? "recursive\\readdir.js" : "recursive/readdir.js",
            "recursive",
            "readdir.js",
          ].sort(compare)
        );
      });
    });

    describe("readdirSync", () => {
      it("should read a directory synchronously", () => {
        const dir = readdirSync(".cargo");
        expect(dir).toEqual(["config.toml"]);
      });

      it("should read a directory with types synchronously", () => {
        const dir = readdirSync(".cargo", {
          withFileTypes: true,
        });
        expect(dir).toEqual([
          {
            name: "config.toml",
            parentPath: ".cargo",
          },
        ]);
        expect(dir[0].isFile()).toBeTruthy();
      });

      it("should read a directory synchronously", () => {
        const dir = readdirSync(".cargo");
        expect(dir).toEqual(["config.toml"]);
      });

      it("should read a directory with recursive synchronously", () => {
        const dir = readdirSync("fixtures/fs/readdir", {
          recursive: true,
        });
        const compare = (a: string | Buffer, b: string | Buffer): number =>
          a >= b ? 1 : -1;
        expect(dir.sort(compare)).toEqual(
          [
            IS_WINDOWS ? "recursive\\readdir.js" : "recursive/readdir.js",
            "recursive",
            "readdir.js",
          ].sort(compare)
        );
      });
    });

    describe("readfile", () => {
      it("should read a file", async () => {
        const buf = await promises.readFile("fixtures/hello.txt");
        const text = buf.toString();
        const base64Text = buf.toString("base64");
        const hexText = buf.toString("hex");

        expect(buf).toBeInstanceOf(Buffer);
        expect(buf).toBeInstanceOf(Uint8Array);
        expect(text).toEqual("hello world!");
        expect(base64Text).toEqual("aGVsbG8gd29ybGQh");
        expect(hexText).toEqual("68656c6c6f20776f726c6421");
      });

      it("should return a string when encoding is provided as option", async () => {
        const text = await promises.readFile("fixtures/hello.txt", {
          encoding: "utf-8",
        });
        expect(typeof text).toEqual("string");
        expect(text).toEqual("hello world!");
      });

      it("should return a string when encoding is provided as string", async () => {
        const text = await promises.readFile("fixtures/hello.txt", "utf-8");
        expect(typeof text).toEqual("string");
        expect(text).toEqual("hello world!");
      });

      it("should return a string when encoding is provided as string with different cases", async () => {
        // @ts-ignore
        const text = await promises.readFile("fixtures/hello.txt", "Utf-8");
        expect(typeof text).toEqual("string");
        expect(text).toEqual("hello world!");
      });
    });

    describe("readfileSync", () => {
      it("should read a file synchronously", () => {
        const buf = readFileSync("fixtures/hello.txt");
        const text = buf.toString();
        const base64Text = buf.toString("base64");
        const hexText = buf.toString("hex");

        expect(buf).toBeInstanceOf(Buffer);
        expect(buf).toBeInstanceOf(Uint8Array);
        expect(text).toEqual("hello world!");
        expect(base64Text).toEqual("aGVsbG8gd29ybGQh");
        expect(hexText).toEqual("68656c6c6f20776f726c6421");
      });

      it("should return a string when encoding is provided as option synchronously", () => {
        const text = readFileSync("fixtures/hello.txt", {
          encoding: "utf-8",
        });
        expect(typeof text).toEqual("string");
        expect(text).toEqual("hello world!");
      });

      it("should return a string when encoding is provided as string synchronously", () => {
        const text = readFileSync("fixtures/hello.txt", "utf-8");
        expect(typeof text).toEqual("string");
        expect(text).toEqual("hello world!");
      });

      it("should return a string when encoding is provided as string with different cases synchronously", async () => {
        // @ts-ignore
        const text = readFileSync("fixtures/hello.txt", "Utf-8");
        expect(typeof text).toEqual("string");
        expect(text).toEqual("hello world!");
      });
    });

    describe("mkdtemp", () => {
      it("should create a temporary directory with a given prefix", async () => {
        // Create a temporary directory with the given prefix
        const prefix = "test-";
        const dirPath = await promises.mkdtemp(path.join(os.tmpdir(), prefix));

        // Check that the directory exists
        const dirExists = await promises
          .stat(dirPath)
          .then(() => true)
          .catch(() => false);
        expect(dirExists).toBeTruthy();

        // Check that the directory has the correct prefix
        const dirPrefix = path.basename(dirPath).slice(0, prefix.length);
        expect(dirPrefix).toEqual(prefix);

        // Clean up the temporary directory
        await promises.rmdir(dirPath);
      });
    });

    describe("mkdtempSync", () => {
      it("should create a temporary directory with a given prefix synchronously", () => {
        // Create a temporary directory with the given prefix
        const prefix = "test-";
        const dirPath = mkdtempSync(path.join(os.tmpdir(), prefix));

        // Check that the directory exists
        const dirExists = statSync(dirPath);
        expect(dirExists).toBeTruthy();

        // Check that the directory has the correct prefix
        const dirPrefix = path.basename(dirPath).slice(0, prefix.length);
        expect(dirPrefix).toEqual(prefix);

        // Clean up the temporary directory
        rmdirSync(dirPath);
      });
    });

    describe("mkdir", () => {
      it("should create a directory with the given path", async () => {
        const dirPath = await promises.mkdtemp(
          path.join(os.tmpdir(), "test/test-")
        );

        //non recursive should reject
        await expect(promises.mkdir(dirPath)).rejects.toThrow(/dir/);

        await promises.mkdir(dirPath, { recursive: true });

        // Helper function to check if directory exists
        const checkDirExists = async (dirPath: string) => {
          return await promises
            .stat(dirPath)
            .then(() => true)
            .catch(() => false);
        };

        // Check that the directory exists
        const dirExists = await checkDirExists(dirPath);
        expect(dirExists).toBeTruthy();

        await promises.rmdir(dirPath, { recursive: true });

        await promises.mkdir(`${dirPath}/./`, { recursive: true });

        // Check that the directory exists
        const dirExists2 = await checkDirExists(dirPath);
        expect(dirExists2).toBeTruthy();

        // Clean up the directory
        await promises.rmdir(dirPath, { recursive: true });
      });
    });

    describe("mkdirSync", () => {
      it("should create a directory with the given path synchronously", () => {
        const dirPath = mkdtempSync(path.join(os.tmpdir(), "test/test-"));

        //non recursive should reject
        expect(() => mkdirSync(dirPath)).toThrow(
          IS_WINDOWS ? /Can\'t create dir/ : /[fF]ile.*exists/
        );

        mkdirSync(dirPath, { recursive: true });

        // Check that the directory exists
        const dirExists = statSync(dirPath);
        expect(dirExists).toBeTruthy();

        // Clean up the directory
        rmdirSync(dirPath, { recursive: true });
      });
    });

    describe("writeFile", () => {
      it("should write a file", async () => {
        const tmpDir = await promises.mkdtemp(path.join(os.tmpdir(), "test-"));
        const filePath = path.join(tmpDir, "test");
        const fileContents = "hello";
        await promises.writeFile(filePath, fileContents);

        const contents = (await promises.readFile(filePath)).toString();

        expect(fileContents).toEqual(contents);

        await promises.rmdir(tmpDir, { recursive: true });
      });
    });

    describe("writeFile synchronously", () => {
      it("should write a file", () => {
        const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
        const filePath = path.join(tmpDir, "test");
        const fileContents = "hello";
        writeFileSync(filePath, fileContents);

        const contents = readFileSync(filePath).toString();

        expect(fileContents).toEqual(contents);

        rmdirSync(tmpDir, { recursive: true });
      });
    });

    describe("rm", () => {
      it("should delete file and directory", async () => {
        const tmpDir = await promises.mkdtemp(path.join(os.tmpdir(), "test-"));
        const filePath = path.join(tmpDir, "test");
        const fileContents = "hello";
        await promises.writeFile(filePath, fileContents);

        const contents = (await promises.readFile(filePath)).toString();
        expect(fileContents).toEqual(contents);

        // Should delete file
        await promises.rm(filePath, { recursive: true });
        await expect(promises.access(filePath)).rejects.toThrow(
          /[Nn]o such file or directory/
        );

        // Check dir still exists and then delete it
        await promises.access(tmpDir);
        await promises.rm(tmpDir, { recursive: true });
        await expect(promises.access(filePath)).rejects.toThrow(
          /[Nn]o such file or directory/
        );
      });
      it("should throw an error if file does not exists", async () => {
        const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
        const filePath = path.join(tmpDir, "test");

        await expect(promises.rm(filePath, {})).rejects.toThrow(
          IS_WINDOWS ? /\(os error 2\)/ : /[Nn]o such file or directory/
        );
      });
      it("should not throw an error if file does not exists and force is used", async () => {
        const tmpDir = await promises.mkdtemp(path.join(os.tmpdir(), "test-"));
        const filePath = path.join(tmpDir, "test");

        await expect(promises.access(filePath)).rejects.toThrow(
          /[Nn]o such file or directory/
        );

        // Should not throw an exception since it does not exists
        await promises.rm(filePath, { force: true, recursive: true });
      });
    });
    describe("rmSync", () => {
      it("should delete file and directory with rm synchronously", async () => {
        const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
        const filePath = path.join(tmpDir, "test");
        const fileContents = "hello";
        await promises.writeFile(filePath, fileContents);

        const contents = readFileSync(filePath).toString();

        expect(fileContents).toEqual(contents);

        // Should delete file
        rmSync(filePath, { recursive: true });
        expect(() => accessSync(filePath)).toThrow(
          /[Nn]o such file or directory/
        );

        // Check dir still exists and then delete it
        accessSync(tmpDir);
        rmSync(tmpDir, { recursive: true });
        expect(() => accessSync(tmpDir)).toThrow(
          /[Nn]o such file or directory/
        );
      });
      it("should throw an error if file does not exists with rm synchronously", async () => {
        const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
        const filePath = path.join(tmpDir, "test");

        expect(() => rmSync(filePath, {})).toThrow(
          IS_WINDOWS ? /\(os error 2\)/ : /[Nn]o such file or directory/
        );
      });
      it("should not throw an error if file does not exists and force is used with rm synchronously", async () => {
        const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
        const filePath = path.join(tmpDir, "test");

        expect(() => accessSync(filePath)).toThrow(
          /[Nn]o such file or directory/
        );

        // Should not throw an exception since it does not exists
        rmSync(filePath, { force: true, recursive: true });
      });
    });

    describe("access", () => {
      it("should access a file", async () => {
        const filePath = "fixtures/hello.txt";
        await promises.access(filePath);
      });

      it("should throw if not proper permissions", async () => {
        const filePath = "fixtures/hello.txt";
        await expect(promises.access(filePath, constants.X_OK)).rejects.toThrow(
          /[pP]ermission denied/
        );
      });

      it("should throw if not exists", async () => {
        const filePath = "fixtures/nothing";
        await expect(promises.access(filePath)).rejects.toThrow(
          /[nN]o such file or directory/
        );
      });

      it("should access a file", async () => {
        const filePath = "fixtures/hello.txt";
        await promises.access(filePath);
      });
    });

    describe("accessSync", () => {
      it("should access a file synchronously", () => {
        const filePath = "fixtures/hello.txt";
        accessSync(filePath);
      });

      it("should throw if not proper permissions synchronously", () => {
        const filePath = "fixtures/hello.txt";
        expect(() => accessSync(filePath, constants.X_OK)).toThrow(
          /[pP]ermission denied/
        );
      });

      it("should throw if not exists synchronously", () => {
        const filePath = "fixtures/nothing";
        expect(() => accessSync(filePath)).toThrow(
          /[Nn]o such file or directory/
        );
      });
    });
  });
});

describe("rename", () => {
  it("should rename a directory", async () => {
    const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "test-"));
    const oldPath = path.join(tmpDir, "old");
    const newPath = path.join(tmpDir, "new");

    await fs.mkdir(oldPath);
    await fs.rename(oldPath, newPath);

    const oldDirExists = await checkDirExists(oldPath);
    const newDirExists = await checkDirExists(newPath);

    expect(oldDirExists).toBeFalsy();
    expect(newDirExists).toBeTruthy();

    // Cleanup
    await fs.rmdir(tmpDir, { recursive: true });
  });

  it("should throw error if source doesn't exist", async () => {
    const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "test-"));
    const oldPath = path.join(tmpDir, "nonexistent");
    const newPath = path.join(tmpDir, "new");

    await expect(fs.rename(oldPath, newPath)).rejects.toThrow(
      /[Nn]o such file or directory/
    );

    await fs.rmdir(tmpDir, { recursive: true });
  });
});

describe("renameSync", () => {
  it("should rename a directory synchronously", () => {
    const tmpDir = defaultFsImport.mkdtempSync(path.join(os.tmpdir(), "test-"));
    const oldPath = path.join(tmpDir, "old");
    const newPath = path.join(tmpDir, "new");

    defaultFsImport.mkdirSync(oldPath);
    defaultFsImport.renameSync(oldPath, newPath);

    // Check if old path doesn't exist (should throw)
    expect(() => defaultFsImport.statSync(oldPath)).toThrow(
      /[Nn]o such file or directory/
    );

    // Check if new path exists and is a directory
    const newDirStat = defaultFsImport.statSync(newPath);
    expect(newDirStat.isDirectory()).toBeTruthy();

    // Cleanup
    defaultFsImport.rmdirSync(tmpDir, { recursive: true });
  });

  it("should throw error if source doesn't exist synchronously", () => {
    const tmpDir = defaultFsImport.mkdtempSync(path.join(os.tmpdir(), "test-"));
    const oldPath = path.join(tmpDir, "nonexistent");
    const newPath = path.join(tmpDir, "new");

    expect(() => defaultFsImport.renameSync(oldPath, newPath)).toThrow(
      /[Nn]o such file or directory/
    );

    defaultFsImport.rmdirSync(tmpDir, { recursive: true });
  });
});
