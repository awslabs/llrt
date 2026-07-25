import { runSuite } from "../_harness-test262.js";
import { runTestDynamic } from "./intl402.harness.js";

runSuite(import.meta.url, runTestDynamic);
