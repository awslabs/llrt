import { runTestDynamic } from "./WebCryptoAPI.harness.js";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const SKIP_FILES = [
  "cfrg_curves_bits_curve25519.https.any.js", // ReferenceError: define_tests_25519 is not defined
  "cfrg_curves_bits_curve448.https.any.js", // ReferenceError: define_tests_448 is not defined
  "cfrg_curves_bits_curve448.tentative.https.any.js", // ReferenceError: define_tests_448 is not defined
  "cfrg_curves_keys_curve25519.https.any.js", // ReferenceError: define_tests_25519 is not defined
  "cfrg_curves_keys_curve448.https.any.js", // ReferenceError: define_tests_448 is not defined
  "cfrg_curves_keys_curve448.tentative.https.any.js", // ReferenceError: define_tests_448 is not defined
  "derive_key_and_encrypt.https.any.js", // ReferenceError: define_tests is not defined
  "derived_bits_length.https.any.js", // ReferenceError: define_tests is not defined
  "ecdh_bits.https.any.js", // ReferenceError: define_tests is not defined
  "ecdh_keys.https.any.js", // ReferenceError: define_tests is not defined
  "hkdf.https.any.js", // ReferenceError: define_tests is not defined
  "pbkdf2.https.any.js", // ReferenceError: define_tests is not defined
];

const __filename = fileURLToPath(import.meta.url);
const basename = path.basename(__filename);
const subDir = basename
  .replace(/\.test\.[jt]s$/, "")
  .split(".")
  .join(path.sep);

const CWD = process.cwd();
const baseDir = path.join(CWD, "wpt");
const targetDir = path.join(baseDir, subDir);

const testFiles = fs
  .readdirSync(targetDir)
  .filter((file) => file.endsWith(".any.js"));

describe(subDir, () => {
  for (const file of testFiles) {
    if (!SKIP_FILES.includes(file)) {
      it(`should pass ${file} tests`, (done) => {
        const filePath = path.join(targetDir, file);
        const sourceCode = fs.readFileSync(filePath, "utf8");
        runTestDynamic(sourceCode, done);
      });
    }
  }
});
