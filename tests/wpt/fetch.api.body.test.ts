import { runSuite } from "./_harness-wpt.js";
import { runTestDynamic } from "./fetch.harness.js";

runSuite(import.meta.url, runTestDynamic);
