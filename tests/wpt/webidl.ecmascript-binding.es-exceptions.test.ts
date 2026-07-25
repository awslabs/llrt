import { runSuite } from "./_harness-wpt.js";
import { runTestDynamic } from "./webidl-harness.js";

runSuite(import.meta.url, runTestDynamic);
