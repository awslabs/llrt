import { existsSync } from "node:fs";
console.log(existsSync("./test.js"));

import { add } from "calc";
console.log(add(1, 2));

import { getHeapStatistics } from "node:v8";
console.log(getHeapStatistics());
