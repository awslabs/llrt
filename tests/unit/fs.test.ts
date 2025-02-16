import fs from "fs/promises";
import defaultFsImport from "fs";
import * as namedFsImport from "fs";
import path from "path";
import os from "os";
import { platform } from "os";
const IS_WINDOWS = platform() === "win32";

describe("readdir", () => {
  it("should read a directory", async () => {
    const dir = await fs.readdir(".cargo");
    expect(dir).toEqual(["config.toml"]);
  });

  it("should read a directory with types", async () => {
    const dir = await fs.readdir(".cargo", { withFileTypes: true });
    expect(dir).toEqual([
      {
        name: "config.toml",
        parentPath: ".cargo",
      },
    ]);
    expect(dir[0].isFile()).toBeTruthy();
  });

  it("should read a directory with types", async () => {
    const dir = await fs.readdir(".cargo/", { withFileTypes: true });
    expect(dir).toEqual([
      {
        name: "config.toml",
        parentPath: ".cargo",
      },
    ]);
    expect(dir[0].isFile()).toBeTruthy();
  });

  it("should read a directory using default import", async () => {
    const dir = await defaultFsImport.promises.readdir(".cargo");
    expect(dir).toEqual(["config.toml"]);
  });

  it("should read a directory using named import", async () => {
    const dir = await namedFsImport.promises.readdir(".cargo");
    expect(dir).toEqual(["config.toml"]);
  });

  it("should read a directory with recursive", async () => {
    const dir = await fs.readdir("fixtures/fs/readdir", { recursive: true });
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
    const dir = defaultFsImport.readdirSync(".cargo");
    expect(dir).toEqual(["config.toml"]);
  });

  it("should read a directory with types synchronously", () => {
    const dir = defaultFsImport.readdirSync(".cargo", { withFileTypes: true });
    expect(dir).toEqual([
      {
        name: "config.toml",
        parentPath: ".cargo",
      },
    ]);
    expect(dir[0].isFile()).toBeTruthy();
  });

  it("should read a directory using default import synchronously", () => {
    const dir = defaultFsImport.readdirSync(".cargo");
    expect(dir).toEqual(["config.toml"]);
  });

  it("should read a directory using named import synchronously", () => {
    const dir = namedFsImport.readdirSync(".cargo");
    expect(dir).toEqual(["config.toml"]);
  });

  it("should read a directory with recursive synchronously", () => {
    const dir = defaultFsImport.readdirSync("fixtures/fs/readdir", {
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
    const buf = await fs.readFile("fixtures/hello.txt");
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
    const text = await fs.readFile("fixtures/hello.txt", { encoding: "utf-8" });
    expect(typeof text).toEqual("string");
    expect(text).toEqual("hello world!");
  });

  it("should return a string when encoding is provided as string", async () => {
    const text = await fs.readFile("fixtures/hello.txt", "utf-8");
    expect(typeof text).toEqual("string");
    expect(text).toEqual("hello world!");
  });

  it("should return a string when encoding is provided as string with different cases", async () => {
    // @ts-ignore
    const text = await fs.readFile("fixtures/hello.txt", "Utf-8");
    expect(typeof text).toEqual("string");
    expect(text).toEqual("hello world!");
  });
});

describe("readfileSync", () => {
  it("should read a file synchronously", () => {
    const buf = defaultFsImport.readFileSync("fixtures/hello.txt");
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
    const text = defaultFsImport.readFileSync("fixtures/hello.txt", {
      encoding: "utf-8",
    });
    expect(typeof text).toEqual("string");
    expect(text).toEqual("hello world!");
  });

  it("should return a string when encoding is provided as string synchronously", () => {
    const text = defaultFsImport.readFileSync("fixtures/hello.txt", "utf-8");
    expect(typeof text).toEqual("string");
    expect(text).toEqual("hello world!");
  });

  it("should return a string when encoding is provided as string with different cases synchronously", async () => {
    // @ts-ignore
    const text = defaultFsImport.readFileSync("fixtures/hello.txt", "Utf-8");
    expect(typeof text).toEqual("string");
    expect(text).toEqual("hello world!");
  });
});

describe("mkdtemp", () => {
  it("should create a temporary directory with a given prefix", async () => {
    // Create a temporary directory with the given prefix
    const prefix = "test-";
    const dirPath = await fs.mkdtemp(path.join(os.tmpdir(), prefix));

    // Check that the directory exists
    const dirExists = await fs
      .stat(dirPath)
      .then(() => true)
      .catch(() => false);
    expect(dirExists).toBeTruthy();

    // Check that the directory has the correct prefix
    const dirPrefix = path.basename(dirPath).slice(0, prefix.length);
    expect(dirPrefix).toEqual(prefix);

    // Clean up the temporary directory
    await fs.rmdir(dirPath);
  });
});

describe("mkdtempSync", () => {
  it("should create a temporary directory with a given prefix synchronously", () => {
    // Create a temporary directory with the given prefix
    const prefix = "test-";
    const dirPath = defaultFsImport.mkdtempSync(path.join(os.tmpdir(), prefix));

    // Check that the directory exists
    const dirExists = defaultFsImport.statSync(dirPath);
    expect(dirExists).toBeTruthy();

    // Check that the directory has the correct prefix
    const dirPrefix = path.basename(dirPath).slice(0, prefix.length);
    expect(dirPrefix).toEqual(prefix);

    // Clean up the temporary directory
    defaultFsImport.rmdirSync(dirPath);
  });
});

describe("mkdir", () => {
  it("should create a directory with the given path", async () => {
    const dirPath = await fs.mkdtemp(path.join(os.tmpdir(), "test/test-"));

    //non recursive should reject
    await expect(fs.mkdir(dirPath)).rejects.toThrow(/dir/);

    await fs.mkdir(dirPath, { recursive: true });

    // Helper function to check if directory exists
    const checkDirExists = async (dirPath: string) => {
      return await fs
        .stat(dirPath)
        .then(() => true)
        .catch(() => false);
    };

    // Check that the directory exists
    const dirExists = await checkDirExists(dirPath);
    expect(dirExists).toBeTruthy();

    await fs.rmdir(dirPath, { recursive: true });

    await fs.mkdir(`${dirPath}/./`, { recursive: true });

    // Check that the directory exists
    const dirExists2 = await checkDirExists(dirPath);
    expect(dirExists2).toBeTruthy();

    // Clean up the directory
    await fs.rmdir(dirPath, { recursive: true });
  });
});

describe("mkdirSync", () => {
  it("should create a directory with the given path synchronously", () => {
    const dirPath = defaultFsImport.mkdtempSync(
      path.join(os.tmpdir(), "test/test-")
    );

    //non recursive should reject
    expect(() => defaultFsImport.mkdirSync(dirPath)).toThrow(
      IS_WINDOWS ? /Can\'t create dir/ : /[fF]ile.*exists/
    );

    defaultFsImport.mkdirSync(dirPath, { recursive: true });

    // Check that the directory exists
    const dirExists = defaultFsImport.statSync(dirPath);
    expect(dirExists).toBeTruthy();

    // Clean up the directory
    defaultFsImport.rmdirSync(dirPath, { recursive: true });
  });
});

describe("writeFile", () => {
  it("should write a file", async () => {
    const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");
    const fileContents = "hello";
    await fs.writeFile(filePath, fileContents);

    const contents = (await fs.readFile(filePath)).toString();

    expect(fileContents).toEqual(contents);

    await fs.rmdir(tmpDir, { recursive: true });
  });
});

describe("writeFile synchronously", () => {
  it("should write a file", () => {
    const tmpDir = defaultFsImport.mkdtempSync(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");
    const fileContents = "hello";
    defaultFsImport.writeFileSync(filePath, fileContents);

    const contents = defaultFsImport.readFileSync(filePath).toString();

    expect(fileContents).toEqual(contents);

    defaultFsImport.rmdirSync(tmpDir, { recursive: true });
  });
});

describe("rm", () => {
  it("should delete file and directory", async () => {
    const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");
    const fileContents = "hello";
    await fs.writeFile(filePath, fileContents);

    const contents = (await fs.readFile(filePath)).toString();
    expect(fileContents).toEqual(contents);

    // Should delete file
    await fs.rm(filePath, { recursive: true });
    await expect(fs.access(filePath)).rejects.toThrow(
      /[Nn]o such file or directory/
    );

    // Check dir still exists and then delete it
    await fs.access(tmpDir);
    await fs.rm(tmpDir, { recursive: true });
    await expect(fs.access(filePath)).rejects.toThrow(
      /[Nn]o such file or directory/
    );
  });
  it("should throw an error if file does not exists", async () => {
    const tmpDir = defaultFsImport.mkdtempSync(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");

    await expect(fs.rm(filePath, {})).rejects.toThrow(
      IS_WINDOWS ? /\(os error 2\)/ : /[Nn]o such file or directory/
    );
  });
  it("should not throw an error if file does not exists and force is used", async () => {
    const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");

    await expect(fs.access(filePath)).rejects.toThrow(
      /[Nn]o such file or directory/
    );

    // Should not throw an exception since it does not exists
    await fs.rm(filePath, { force: true, recursive: true });
  });
});
describe("rmSync", () => {
  it("should delete file and directory with rm synchronously", async () => {
    const tmpDir = defaultFsImport.mkdtempSync(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");
    const fileContents = "hello";
    await fs.writeFile(filePath, fileContents);

    const contents = defaultFsImport.readFileSync(filePath).toString();

    expect(fileContents).toEqual(contents);

    // Should delete file
    defaultFsImport.rmSync(filePath, { recursive: true });
    expect(() => defaultFsImport.accessSync(filePath)).toThrow(
      /[Nn]o such file or directory/
    );

    // Check dir still exists and then delete it
    defaultFsImport.accessSync(tmpDir);
    defaultFsImport.rmSync(tmpDir, { recursive: true });
    expect(() => defaultFsImport.accessSync(tmpDir)).toThrow(
      /[Nn]o such file or directory/
    );
  });
  it("should throw an error if file does not exists with rm synchronously", async () => {
    const tmpDir = defaultFsImport.mkdtempSync(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");

    expect(() => defaultFsImport.rmSync(filePath, {})).toThrow(
      IS_WINDOWS ? /\(os error 2\)/ : /[Nn]o such file or directory/
    );
  });
  it("should not throw an error if file does not exists and force is used with rm synchronously", async () => {
    const tmpDir = defaultFsImport.mkdtempSync(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");

    expect(() => defaultFsImport.accessSync(filePath)).toThrow(
      /[Nn]o such file or directory/
    );

    // Should not throw an exception since it does not exists
    defaultFsImport.rmSync(filePath, { force: true, recursive: true });
  });
});

describe("access", () => {
  it("should access a file", async () => {
    const filePath = "fixtures/hello.txt";
    await fs.access(filePath);
  });

  it("should throw if not proper permissions", async () => {
    const filePath = "fixtures/hello.txt";
    await expect(fs.access(filePath, fs.constants.X_OK)).rejects.toThrow(
      /[pP]ermission denied/
    );
  });

  it("should throw if not exists", async () => {
    const filePath = "fixtures/nothing";
    await expect(fs.access(filePath)).rejects.toThrow(
      /[nN]o such file or directory/
    );
  });

  it("should access a file using default import", async () => {
    const filePath = "fixtures/hello.txt";
    await defaultFsImport.promises.access(filePath);
  });

  it("should access a file using named import", async () => {
    const filePath = "fixtures/hello.txt";
    await namedFsImport.promises.access(filePath);
  });
});

describe("accessSync", () => {
  it("should access a file synchronously", () => {
    const filePath = "fixtures/hello.txt";
    defaultFsImport.accessSync(filePath);
  });

  it("should throw if not proper permissions synchronously", () => {
    const filePath = "fixtures/hello.txt";
    expect(() =>
      defaultFsImport.accessSync(filePath, fs.constants.X_OK)
    ).toThrow(/[pP]ermission denied/);
  });

  it("should throw if not exists synchronously", () => {
    const filePath = "fixtures/nothing";
    expect(() => defaultFsImport.accessSync(filePath)).toThrow(
      /[Nn]o such file or directory/
    );
  });
});
