import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "argon2.tentative.https.any.js", // ReferenceError: define_tests is not defined
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
]);
