import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./streams.harness.js";

runSuite(import.meta.url, runTestDynamic);
