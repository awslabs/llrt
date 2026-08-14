import { runSuite, runTestDynamic } from "./FileAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  [
    "Blob-constructor.any.js",
    [
      "[Passing a FrozenArray as the blobParts array should work (FrozenArray<MessagePort>).] MessageChannel is not defined",
    ],
  ],
]);
