import { runSuite } from "./_harness-wpt.js";
import { runTestDynamic } from "./FileAPI.harness.js";

runSuite(import.meta.url, runTestDynamic);
