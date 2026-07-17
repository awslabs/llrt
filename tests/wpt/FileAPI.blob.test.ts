import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./FileAPI.harness.js";

runSuite(import.meta.url, runTestDynamic);
