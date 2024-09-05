import fs from "fs/promises";
import path from "path";

//read all files in ./bundle/lrt that ends with .lrt
async function readFiles(bytecodeDir) {
  const fileEntries = await fs.readdir(bytecodeDir, {
    recursive: true,
    withFileTypes: true,
  });
  const files = fileEntries.reduce((acc, { name, parentPath }) => {
    if (name.endsWith(".lrt")) {
      acc.push(path.join(parentPath, name));
    }

    return acc;
  }, []);
  files.sort((a, b) => a.localeCompare(b));
  return files;
}

async function readFileData(bytecodeDir, files) {
  return await Promise.all(
    files.map(async (file) => {
      const data = await fs.readFile(file);
      const { name, dir } = path.parse(path.relative(bytecodeDir, file));
      return [`${dir ? `${dir}/` : ""}${name}`, data];
    })
  );
}

async function buildFileIndex(source, target, fileData, writeRaw) {
  const uint32Buffer = (length) => {
    const buffer = Buffer.alloc(4);
    buffer.writeUInt32LE(length);
    return buffer;
  };

  const uint16Buffer = (length) => {
    const buffer = Buffer.alloc(2);
    buffer.writeUInt16LE(length);
    return buffer;
  };

  const sourceData = await fs.readFile(source);
  const packageIndexList = [];
  let offset = 0;
  const bytecodeData = [];

  for (let [name, data] of fileData) {
    if (name.startsWith("lrt") || name.startsWith("llrt")) {
      name = `${name}.js`;
    }
    const nameLengthBuffer = uint16Buffer(name.length);
    const nameBuffer = Buffer.from(name);

    const bytecodeSizeBuffer = uint32Buffer(data.length);
    const bytecodeOffsetBuffer = uint32Buffer(offset);

    packageIndexList.push(
      Buffer.concat([
        nameLengthBuffer,
        nameBuffer,
        bytecodeOffsetBuffer,
        bytecodeSizeBuffer,
      ])
    );

    offset += data.length;
    bytecodeData.push(data);
  }

  const allBytecodeData = Buffer.concat(bytecodeData);

  const packageCount = fileData.length;
  const bytecodePosition = writeRaw ? 0 : sourceData.length;
  const packageIndexPosition = bytecodePosition + allBytecodeData.length;

  const metadataBuffer = Buffer.concat([
    uint32Buffer(packageCount),
    uint32Buffer(bytecodePosition),
    uint32Buffer(packageIndexPosition),
    Buffer.from("lrt"),
  ]);

  const packageIndexBuffer = Buffer.concat(packageIndexList);

  const finalBuffer = Buffer.concat([
    ...(writeRaw ? [] : [sourceData]),
    allBytecodeData,
    packageIndexBuffer,
    metadataBuffer,
  ]);

  console.log("Embedded size:", allBytecodeData.length / 1024, "kB");

  await fs.writeFile(target, finalBuffer);
  if (!writeRaw) {
    await fs.chmod(target, 0o755);
  }
}

const [bytecodeDir, source, target, rawArg] = process.argv.slice(2);
const writeRaw = rawArg == "-r" || rawArg == "--raw";

if (!bytecodeDir || !source || !target) {
  console.error(
    `No source or target specified, use:\n${path.basename(process.argv[0])} ${path.basename(process.argv[1])} {bytecode_directory} {input_target} {output_target}`
  );
  process.exit(1);
}

console.log("Reading files...");
const files = await readFiles(bytecodeDir);
console.log("Reading file data...");
const filesContents = await readFileData(bytecodeDir, files);
await buildFileIndex(source, target, filesContents, writeRaw);
