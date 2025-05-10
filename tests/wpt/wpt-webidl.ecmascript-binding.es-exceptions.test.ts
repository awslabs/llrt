import { runTest } from "./runTest.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt`;

const testFiles = [
  "DOMException-constants.any.js",
  "DOMException-constructor-and-prototype.any.js",
  "DOMException-constructor-behavior.any.js",
  "DOMException-custom-bindings.any.js",
];

describe("es-exceptions", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      runTest(require(`./webidl/ecmascript-binding/es-exceptions/${file}`).default, done);
    });
  }
});
