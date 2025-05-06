// Simple test script for the executable functionality
console.log("Hello from LLRT executable test!");
console.log(
  "This file is used to test the --executable flag for 'llrt compile'"
);

// Print command line arguments
console.log("Command line arguments:", process.argv);

// Test some basic JavaScript features to ensure they work in the executable
interface RuntimeInfo {
  name: string;
  version: string;
  features: string[];
}

const runtimeInfo: RuntimeInfo = {
  name: "LLRT",
  version: process.env.LLRT_VERSION || "test",
  features: [
    "Self-contained executables",
    "Bytecode compilation",
    "Fast JavaScript runtime",
  ],
};

console.log("Runtime info:", runtimeInfo);
console.log("Features:", runtimeInfo.features.join(", "));

// Test async functions
async function testAsync(): Promise<void> {
  return new Promise<void>((resolve) => {
    setTimeout(() => {
      console.log("Async operation completed");
      resolve();
    }, 100);
  });
}

// Run async test and exit with custom code
(async (): Promise<never> => {
  await testAsync();
  // Exit with custom code to test proper process exit handling
  process.exit(42);
})(); 