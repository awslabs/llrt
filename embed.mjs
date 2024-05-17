import fs from "fs/promises";
import path from "path";

const BUNDLE_DIR = "./bundle/lrt";

//read all files in ./bundle/js that ends with .lrt
async function readFiles() {
  const fileEntries = await fs.readdir(BUNDLE_DIR, {
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

async function readFileData(files) {
  return await Promise.all(
    files.map(async (file) => {
      const data = await fs.readFile(file);
      const { name, dir } = path.parse(path.relative(BUNDLE_DIR, file));
      return [`${dir ? `${dir}/` : ""}${name}`, data];
    })
  );
}

async function buildFileIndex(source, target, fileData) {
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
  const dataBuffers = [];
  let offset = 0;
  let indexBuffers = [];
  for (let [name, data] of fileData) {
    dataBuffers.push(data);

    if(name.startsWith("llrt-chunk-")){
      name = `${name}.js`
    }

    const nameLengthBuffer = uint16Buffer(name.length);
    const nameBuffer = Buffer.from(name);
    const bytecodeSizeBuffer = uint32Buffer(data.length);
    const bytecodeOffsetBuffer = uint32Buffer(offset);

    indexBuffers.push(
      Buffer.concat([
        nameLengthBuffer,
        nameBuffer,
        bytecodeSizeBuffer,
        bytecodeOffsetBuffer,
      ])
    );

    offset += data.length;
  }

  const dataBuffer = Buffer.concat(dataBuffers);

  const packageCount = fileData.length;
  const bytecodePosition = sourceData.length;
  const packageIndexPosition = sourceData.length + dataBuffer.length;

  //[u32 package_count][u32 bytecode_pos][u32 package_index_pos]
  const metadataBuffer = Buffer.concat([
    uint32Buffer(packageCount),
    uint32Buffer(bytecodePosition),
    uint32Buffer(packageIndexPosition),
  ]);

  const signatureBuffer = Buffer.from("lrt");

  let allIndexBuffers = Buffer.concat(indexBuffers);

  console.log("index buffer size: ", allIndexBuffers.length);

  const finalBuffer = Buffer.concat([
    sourceData,
    dataBuffer,
    allIndexBuffers,
    metadataBuffer,
    signatureBuffer,
  ]);

  await fs.writeFile(target, finalBuffer);
  await fs.chmod(target, 0o755);
}

const source = process.argv[2];
const target = process.argv[3];
if (!source || !target) {
  console.error(
    `No source or target specified, use:\n${path.basename(process.argv[0])} ${path.basename(process.argv[1])} {input_target} {output_target}`
  );
  process.exit(1);
}

console.log("Reading files...");
const files = await readFiles();
console.log("Reading file data...");
const filesContents = await readFileData(files);
await buildFileIndex(source, target, filesContents);
