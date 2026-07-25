import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  makeRunner as baseRunner,
  runSuite as baseRunSuite,
  loadMetaScripts,
} from "./_harness-util.js";

const CWD = process.cwd();
const TEST_DIR = path.join(CWD, "bundle", "js", "__tests__", "wpt");

export function makeRunner(config) {
  return baseRunner(config);
}

export { loadMetaScripts };

export function runSuite(metaUrl, harness, skipFiles = []) {
  return baseRunSuite(metaUrl, harness, skipFiles, {
    filePattern: /\.any\.js$/,
    subDir: deriveSubDir(metaUrl),
  });
}

export function deriveSubDir(metaUrl) {
  const metaPath = fileURLToPath(metaUrl);
  const relativePath = path.relative(TEST_DIR, metaPath);

  const match = relativePath.match(/^(.+)\.test\.[jt]s$/);

  return path.join(...match[1].split("."));
}
