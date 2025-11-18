import { spawn } from "node:child_process";

export const spawnCapture = (
  cmd: string,
  args: string[] = [],
  options?: { env?: Record<string, string> }
): Promise<{ code: number; stdout: string; stderr: string }> =>
  new Promise((resolve, reject) => {
    let stdout = "";
    let stderr = "";
    const child = spawn(cmd, args, options);
    child.stdout.on("data", (data) => {
      stdout += data.toString();
    });
    child.stderr.on("data", (data) => {
      stderr += data.toString();
    });
    child.on("close", (code) => resolve({ code: code ?? -1, stdout, stderr }));
    child.on("error", reject);
  });
