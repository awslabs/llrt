import defaultImport from "node:fs";
import legacyImport from "fs";

import path from "node:path";
import os from "node:os";
const IS_WINDOWS = os.platform() === "win32";

it("node:fs should be the same as fs", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const {
  constants,
  accessSync,
  readdirSync,
  readFileSync,
  mkdtempSync,
  mkdirSync,
  renameSync,
  rmSync,
  rmdirSync,
  statSync,
  symlinkSync,
  writeFileSync,
  promises,
} = defaultImport;

const {
  access,
  mkdir,
  mkdtemp,
  readdir,
  readFile,
  rename,
  rm,
  rmdir,
  symlink,
  writeFile,
} = promises;

describe("readdir", () => {
  it("should read a directory", async () => {
    const dir = await readdir(".cargo");
    expect(dir).toEqual(["config.toml"]);
  });

  it("should read a directory with types", async () => {
    const dir = await readdir(".cargo", { withFileTypes: true });
    expect(dir).toEqual([
      {
        name: "config.toml",
        parentPath: ".cargo",
      },
    ]);
    expect(dir[0].isFile()).toBeTruthy();
  });

  it("should read a directory with types", async () => {
    const dir = await readdir(".cargo/", { withFileTypes: true });
    expect(dir).toEqual([
      {
        name: "config.toml",
        parentPath: ".cargo",
      },
    ]);
    expect(dir[0].isFile()).toBeTruthy();
  });

  it("should read a directory", async () => {
    const dir = await readdir(".cargo");
    expect(dir).toEqual(["config.toml"]);
  });

  it("should read a directory with recursive", async () => {
    const dir = await readdir("fixtures/fs/readdir", {
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
    const buf = await readFile("fixtures/hello.txt");
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
    const text = await readFile("fixtures/hello.txt", {
      encoding: "utf-8",
    });
    expect(typeof text).toEqual("string");
    expect(text).toEqual("hello world!");
  });

  it("should return a string when encoding is provided as string", async () => {
    const text = await readFile("fixtures/hello.txt", "utf-8");
    expect(typeof text).toEqual("string");
    expect(text).toEqual("hello world!");
  });

  it("should return a string when encoding is provided as string with different cases", async () => {
    // @ts-ignore
    const text = await readFile("fixtures/hello.txt", "Utf-8");
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
    const dirPath = await mkdtemp(path.join(os.tmpdir(), prefix));

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
    await rmdir(dirPath);
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
    const dirPath = await mkdtemp(path.join(os.tmpdir(), "test/test-"));

    //non recursive should reject
    await expect(mkdir(dirPath)).rejects.toThrow(/dir/);

    await mkdir(dirPath, { recursive: true });

    // Check that the directory exists
    const dirExists = await checkDirExists(dirPath);
    expect(dirExists).toBeTruthy();

    await rmdir(dirPath, { recursive: true });

    await mkdir(`${dirPath}/./`, { recursive: true });

    // Check that the directory exists
    const dirExists2 = await checkDirExists(dirPath);
    expect(dirExists2).toBeTruthy();

    // Clean up the directory
    await rmdir(dirPath, { recursive: true });
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
    const tmpDir = await mkdtemp(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");
    const fileContents = "hello";
    await writeFile(filePath, fileContents);

    const contents = (await readFile(filePath)).toString();

    expect(fileContents).toEqual(contents);

    await rmdir(tmpDir, { recursive: true });
  });

  if (!IS_WINDOWS) {
    it("should write file with permissions", async () => {
      const tmpDir = await mkdtemp(path.join(os.tmpdir(), "test-"));
      const filePath = path.join(tmpDir, "test");
      const fileContents = "hello";
      const mode = 0o644;
      await writeFile(filePath, fileContents, { mode });

      const stats = statSync(filePath);
      expect(stats.mode & 0o777).toEqual(mode);

      await rmdir(tmpDir, { recursive: true });
    });
  }
});

describe("writeFileSync", () => {
  it("should write a file", () => {
    const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");
    const fileContents = "hello";
    writeFileSync(filePath, fileContents);

    const contents = readFileSync(filePath).toString();

    expect(fileContents).toEqual(contents);

    rmdirSync(tmpDir, { recursive: true });
  });

  if (!IS_WINDOWS) {
    it("should write file with permissions", async () => {
      const tmpDir = await mkdtemp(path.join(os.tmpdir(), "test-"));
      const filePath = path.join(tmpDir, "test");
      const fileContents = "hello";
      const mode = 0o644;
      writeFileSync(filePath, fileContents, { mode });

      const stats = statSync(filePath);
      expect(stats.mode & 0o777).toEqual(mode);

      rmdirSync(tmpDir, { recursive: true });
    });
  }
});

describe("rm", () => {
  it("should delete file and directory", async () => {
    const tmpDir = await mkdtemp(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");
    const fileContents = "hello";
    await writeFile(filePath, fileContents);

    const contents = (await readFile(filePath)).toString();
    expect(fileContents).toEqual(contents);

    // Should delete file
    await rm(filePath, { recursive: true });
    await expect(access(filePath)).rejects.toThrow(
      /[Nn]o such file or directory/
    );

    // Check dir still exists and then delete it
    await access(tmpDir);
    await rm(tmpDir, { recursive: true });
    await expect(access(filePath)).rejects.toThrow(
      /[Nn]o such file or directory/
    );
  });
  it("should throw an error if file does not exists", async () => {
    const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");

    await expect(rm(filePath, {})).rejects.toThrow(
      IS_WINDOWS ? /\(os error 2\)/ : /[Nn]o such file or directory/
    );
  });
  it("should not throw an error if file does not exists and force is used", async () => {
    const tmpDir = await mkdtemp(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");

    await expect(access(filePath)).rejects.toThrow(
      /[Nn]o such file or directory/
    );

    // Should not throw an exception since it does not exists
    await rm(filePath, { force: true, recursive: true });
  });
});
describe("rmSync", () => {
  it("should delete file and directory with rm synchronously", async () => {
    const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");
    const fileContents = "hello";
    await writeFile(filePath, fileContents);

    const contents = readFileSync(filePath).toString();

    expect(fileContents).toEqual(contents);

    // Should delete file
    rmSync(filePath, { recursive: true });
    expect(() => accessSync(filePath)).toThrow(/[Nn]o such file or directory/);

    // Check dir still exists and then delete it
    accessSync(tmpDir);
    rmSync(tmpDir, { recursive: true });
    expect(() => accessSync(tmpDir)).toThrow(/[Nn]o such file or directory/);
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

    expect(() => accessSync(filePath)).toThrow(/[Nn]o such file or directory/);

    // Should not throw an exception since it does not exists
    rmSync(filePath, { force: true, recursive: true });
  });
});

describe("access", () => {
  it("should access a file", async () => {
    const filePath = "fixtures/hello.txt";
    await access(filePath);
  });

  it("should throw if not proper permissions", async () => {
    const filePath = "fixtures/hello.txt";
    await expect(access(filePath, constants.X_OK)).rejects.toThrow(
      /[pP]ermission denied/
    );
  });

  it("should throw if not exists", async () => {
    const filePath = "fixtures/nothing";
    await expect(access(filePath)).rejects.toThrow(
      /[nN]o such file or directory/
    );
  });

  it("should access a file", async () => {
    const filePath = "fixtures/hello.txt";
    await access(filePath);
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
    expect(() => accessSync(filePath)).toThrow(/[Nn]o such file or directory/);
  });
});

describe("rename", () => {
  it("should rename a directory", async () => {
    const tmpDir = await mkdtemp(path.join(os.tmpdir(), "test-"));
    const oldPath = path.join(tmpDir, "old");
    const newPath = path.join(tmpDir, "new");

    await mkdir(oldPath);
    await rename(oldPath, newPath);

    const oldDirExists = await checkDirExists(oldPath);
    const newDirExists = await checkDirExists(newPath);

    expect(oldDirExists).toBeFalsy();
    expect(newDirExists).toBeTruthy();

    // Cleanup
    await rmdir(tmpDir, { recursive: true });
  });

  it("should throw error if source doesn't exist", async () => {
    const tmpDir = await mkdtemp(path.join(os.tmpdir(), "test-"));
    const oldPath = path.join(tmpDir, "nonexistent");
    const newPath = path.join(tmpDir, "new");

    await expect(rename(oldPath, newPath)).rejects.toThrow(
      /[Nn]o such file or directory/
    );

    await rmdir(tmpDir, { recursive: true });
  });
});

describe("renameSync", () => {
  it("should rename a directory synchronously", () => {
    const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
    const oldPath = path.join(tmpDir, "old");
    const newPath = path.join(tmpDir, "new");

    mkdirSync(oldPath);
    renameSync(oldPath, newPath);

    // Check if old path doesn't exist (should throw)
    expect(() => statSync(oldPath)).toThrow(/[Nn]o such file or directory/);

    // Check if new path exists and is a directory
    const newDirStat = statSync(newPath);
    expect(newDirStat.isDirectory()).toBeTruthy();

    // Cleanup
    rmdirSync(tmpDir, { recursive: true });
  });

  it("should throw error if source doesn't exist synchronously", () => {
    const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
    const oldPath = path.join(tmpDir, "nonexistent");
    const newPath = path.join(tmpDir, "new");

    expect(() => renameSync(oldPath, newPath)).toThrow(
      /[Nn]o such file or directory/
    );

    rmdirSync(tmpDir, { recursive: true });
  });
});

describe("symlink", () => {
  it("should create a symlink", async () => {
    const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "file");
    const linkPath = path.join(tmpDir, "link");

    const expectedContent = "hello world";
    await writeFile(filePath, expectedContent);
    await symlink(filePath, linkPath);

    // Check if new path exists and is a symlink
    const linkStat = statSync(linkPath);
    expect(linkStat.isSymbolicLink()).toBeTruthy();

    const content = await readFile(linkPath, "utf-8");
    expect(content).toBe(expectedContent);

    // Cleanup
    rmdirSync(tmpDir, { recursive: true });
  });
});

describe("symlinkSync", () => {
  it("should create a symlink synchronously", () => {
    const tmpDir = mkdtempSync(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "file");
    const linkPath = path.join(tmpDir, "link");

    const expectedContent = "hello world";
    writeFileSync(filePath, expectedContent);
    symlinkSync(filePath, linkPath);

    // Check if new path exists and is a symlink
    const linkStat = statSync(linkPath);
    expect(linkStat.isSymbolicLink()).toBeTruthy();

    const content = readFileSync(linkPath, "utf-8");
    expect(content).toBe(expectedContent);

    // Cleanup
    rmdirSync(tmpDir, { recursive: true });
  });
});

// Helper function to check if directory exists
const checkDirExists = async (dirPath: string) => {
  return await promises
    .stat(dirPath)
    .then(() => true)
    .catch(() => false);
};
