import fs from "fs/promises";
import defaultFsImport from "fs";
import * as namedFsImport from "fs";
import path from "path";
import os from "os";

describe("readdir", () => {
  it("should read a directory", async () => {
    const dir = await fs.readdir(".cargo");
    assert.deepEqual(dir, ["config.toml"]);
  });

  it("should read a directory with types", async () => {
    const dir = await fs.readdir(".cargo", { withFileTypes: true });
    assert.deepEqual(dir, [{ name: "config.toml" }]);
    assert.equal(dir[0].isFile(), true);
  });

  it("should read a directory using default import", async () => {
    const dir = await defaultFsImport.promises.readdir(".cargo");
    assert.deepEqual(dir, ["config.toml"]);
  });

  it("should read a directory using named import", async () => {
    const dir = await namedFsImport.promises.readdir(".cargo");
    assert.deepEqual(dir, ["config.toml"]);
  });

  it("should read a directory with recursive", async () => {
    const dir = await fs.readdir("fixtures/fs/readdir", { recursive: true });
    const compare = (a: string, b: string) => (a >= b ? 1 : -1);
    assert.deepEqual(
      dir.sort(compare),
      ["recursive/readdir.js", "recursive", "readdir.js"].sort(compare)
    );
  });
});

describe("readdirSync", () => {
  it("should read a directory synchronously", () => {
    const dir = defaultFsImport.readdirSync(".cargo");
    assert.deepEqual(dir, ["config.toml"]);
  });

  it("should read a directory with types synchronously", () => {
    const dir = defaultFsImport.readdirSync(".cargo", { withFileTypes: true });
    assert.deepEqual(dir, [{ name: "config.toml" }]);
    assert.equal(dir[0].isFile(), true);
  });

  it("should read a directory using default import synchronously", () => {
    const dir = defaultFsImport.readdirSync(".cargo");
    assert.deepEqual(dir, ["config.toml"]);
  });

  it("should read a directory using named import synchronously", () => {
    const dir = namedFsImport.readdirSync(".cargo");
    assert.deepEqual(dir, ["config.toml"]);
  });

  it("should read a directory with recursive synchronously", () => {
    const dir = defaultFsImport.readdirSync("fixtures/fs/readdir", {
      recursive: true,
    });
    const compare = (a: string | Buffer, b: string | Buffer): number =>
      a >= b ? 1 : -1;
    assert.deepEqual(
      dir.sort(compare),
      ["recursive/readdir.js", "recursive", "readdir.js"].sort(compare)
    );
  });
});

describe("readfile", () => {
  it("should read a file", async () => {
    const buf = await fs.readFile("fixtures/hello.txt");
    const text = buf.toString();
    const base64Text = buf.toString("base64");
    const hexText = buf.toString("hex");

    assert.ok(buf instanceof Buffer);
    assert.ok(buf instanceof Uint8Array);
    assert.equal(text, "hello world!");
    assert.equal(base64Text, "aGVsbG8gd29ybGQh");
    assert.equal(hexText, "68656c6c6f20776f726c6421");
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
    assert.ok(dirExists);

    // Check that the directory has the correct prefix
    const dirPrefix = path.basename(dirPath).slice(0, prefix.length);
    assert.strictEqual(dirPrefix, prefix);

    // Clean up the temporary directory
    //await fs.rmdir(dirPath);
  });
});

describe("mkdir", () => {
  it("should create a directory with the given path", async () => {
    const dirPath = await fs.mkdtemp(path.join(os.tmpdir(), "test/test-"));

    //non recursive should reject
    expect(fs.mkdir(dirPath)).rejects.toThrow(
      "EEXIST: file already exists, mkdir"
    );

    await fs.mkdir(dirPath, { recursive: true });

    // Check that the directory exists
    const dirExists = await fs
      .stat(dirPath)
      .then(() => true)
      .catch(() => false);
    assert.ok(dirExists);

    // Clean up the directory
    await fs.rmdir(dirPath, { recursive: true });
  });
});

describe("mkdirSync", () => {
  it("should create a directory with the given path synchronously", async () => {
    const dirPath = await fs.mkdtemp(path.join(os.tmpdir(), "test/test-"));

    //non recursive should reject
    expect(() => defaultFsImport.mkdirSync(dirPath)).toThrow(/[fF]ile.*exists/);

    defaultFsImport.mkdirSync(dirPath, { recursive: true });

    // Check that the directory exists
    const dirExists = await fs
      .stat(dirPath)
      .then(() => true)
      .catch(() => false);
    assert.ok(dirExists);

    // Clean up the directory
    await fs.rmdir(dirPath, { recursive: true });
  });
});

describe("writeFile", () => {
  it("should write a file", async () => {
    const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "test-"));
    const filePath = path.join(tmpDir, "test");
    const fileContents = "hello";
    await fs.writeFile(filePath, fileContents);

    const contents = (await fs.readFile(filePath)).toString();

    assert.equal(fileContents, contents);

    await fs.rmdir(tmpDir, { recursive: true });
  });
});

describe("access", () => {
  it("should access a file", async () => {
    const filePath = "fixtures/hello.txt";
    await fs.access(filePath);
  });

  it("should throw if not proper permissions", async () => {
    const filePath = "fixtures/hello.txt";
    assert.rejects(fs.access(filePath, fs.constants.X_OK));
  });

  it("should throw if not exists", async () => {
    const filePath = "fixtures/nothing";
    assert.rejects(fs.access(filePath));
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
