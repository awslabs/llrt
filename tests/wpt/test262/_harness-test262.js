import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  makeRunner as baseRunner,
  runSuite as baseRunSuite,
} from "../_harness-util.js";

const CWD = process.cwd();
const TEST_DIR = path.join(CWD, "bundle", "js", "__tests__", "wpt");

export function makeRunner(config) {
  return baseRunner(config, { allowNoTests: true });
}

export function runSuite(metaUrl, harness, skipFiles = []) {
  return baseRunSuite(metaUrl, harness, skipFiles, {
    filePattern: /\.js$/,
    subDir: deriveTest262SubDir(metaUrl),
  });
}

function deriveTest262SubDir(metaUrl) {
  const metaPath = fileURLToPath(metaUrl);
  const relativePath = path.relative(TEST_DIR, metaPath);

  const match = relativePath.match(/^test262[\\/](.+)\.test\.[jt]s$/);

  return path.join("third_party", "test262", "test", ...match[1].split("."));
}
