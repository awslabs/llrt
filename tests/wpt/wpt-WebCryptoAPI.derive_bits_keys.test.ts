import { runTest } from "./wpt-WebCryptoAPI.harness.js";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt/wpt`;

const testFiles = [
  "cfrg_curves_bits_curve25519.https.any.js",
  "cfrg_curves_bits_curve448.https.any.js",
  "cfrg_curves_keys_curve25519.https.any.js",
  "cfrg_curves_keys_curve448.https.any.js",
  "derive_key_and_encrypt.https.any.js",
  "derived_bits_length.https.any.js",
  "ecdh_bits.https.any.js",
  "ecdh_keys.https.any.js",
  "hkdf.https.any.js",
  "pbkdf2.https.any.js",
];

describe("WebCryptoAPI.derive_bits_keys", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      runTest(
        require(baseDir + "/WebCryptoAPI/derive_bits_keys/" + file).default,
        done
      );
    });
  }
});
