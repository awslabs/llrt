import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./webidl-harness.js";

runSuite(import.meta.url, runTestDynamic);
