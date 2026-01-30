// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
import net from "node:net";
import os from "node:os";
import { spawn, ChildProcess } from "node:child_process";
import path from "node:path";
import { SocketReqMsg } from "./shared";
import { platform } from "node:os";
const IS_WINDOWS = platform() === "win32";
import CircularBuffer from "./CircularBuffer";
// @ts-ignore
import { dimensions } from "llrt:util";

type TestOptions = {
  workerCount?: number;
};

type TestProps = {
  success: boolean;
  started: number;
  ended: number;
};

type TestResult = TestProps & {
  desc: string;
  error: Error | null;
};

type SuiteResult = TestProps & {
  desc: string;
  tests: TestResult[];
  children: SuiteResult[];
  parent: SuiteResult | null;
};

type RootSuite = TestProps & {
  results: SuiteResult[];
  name: string;
  printed: boolean;
};

type WorkerData = {
  writeInProgress: boolean;
  stdOutBuffer: CircularBuffer;
  stdErrBuffer: CircularBuffer;
  completed: boolean;
  childProc?: ChildProcess;
  lastUpdate: number;
  success: boolean;
  connectionTimeout: Timeout | null;
  currentTest: TestResult | null;
  result: SuiteResult | null;
  file: string;
  currentPath: string[];
  currentTimeout: number;
};

class Color {
  private static colorizer =
    (
      color: number | null,
      bgColor: number | null = null,
      style: number | null = null
    ) =>
    (text: string) =>
      `\x1b[${color || bgColor || style}m${text}${Color.RESET}`;

  static GREEN = Color.colorizer(32);
  static RED = Color.colorizer(31);
  static GREEN_BACKGROUND = Color.colorizer(null, 42);
  static RED_BACKGROUND = Color.colorizer(null, 41);
  static DIM = Color.colorizer(null, null, 2);
  static BOLD = Color.colorizer(null, null, 1);
  static CYAN_BOLD = Color.colorizer(36, null, 1);
  static RESET = "\x1b[0m";
}

type TestFailure = {
  error: any;
  desc: string[];
  stdErr: string;
  stdOut: string;
};

class TestServer {
  private static UPDATE_FPS = 15;
  private static UPDATE_INTERVAL_MS = 1000 / TestServer.UPDATE_FPS;
  private static DEFAULT_TIMEOUT_MS =
    parseInt((process.env as any).TEST_TIMEOUT) || 5000;
  private static DEFAULT_PROGRESS_BAR_WIDTH = 24;
  private static SPINNER = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
  private static TESTING_TEXT = " Testing ";
  private static CHECKMARK = "\u2714";
  private static CROSS = "\u2718";
  static ERROR_CODE_SOCKET_ERROR = 1;
  static ERROR_CODE_SOCKET_WRITE_ERROR = 2;
  static ERROR_CODE_PROCESS_ERROR = 4;
  static ERROR_CODE_HANDLE_DATA = 8;

  private server: net.Server | null = null;
  private workerCount: number;
  private workerIdBySocket: Map<net.Socket, number> = new Map();
  private testFiles: string[];
  private testFileNames: string[];
  private fileQueue: string[];
  private filesFailed: Map<string, TestFailure[]>;
  private filesCompleted: Set<string>;
  private completedWorkers: number = 0;
  private workerData: Record<number, WorkerData> = {};
  private workerDataFileInProgress: Map<string, WorkerData> = new Map();
  private results: Map<string, RootSuite> = new Map();
  private totalTests: number = 0;
  private totalSuccess: number = 0;
  private totalSkipped: number = 0;
  private totalFailed: number = 0;
  private totalOnly: number = 0;
  private lastUpdate = 0;
  private updateInterval: Timeout | null = null;
  private spinnerFrameIndex = 0;
  private started = 0;
  private shutdownPending = false;
  private nextWorkerId = 0;
  private activeWorkers = 0;
  private printFinalResults = false;

  constructor(
    testFiles: string[],
    { workerCount = os.availableParallelism() }: TestOptions = {}
  ) {
    this.fileQueue = [...testFiles];
    this.testFiles = [...testFiles];
    this.testFileNames = testFiles.map((file) => path.basename(file));
    this.filesFailed = new Map();
    this.filesCompleted = new Set();
    this.workerCount = Math.min(workerCount, testFiles.length);
  }

  public async start() {
    if (this.testFiles.length === 0) {
      this.printResults();
      this.shutdown();
      return;
    }
    this.started = performance.now();
    const server = net.createServer((socket) =>
      this.handleSocketConnected(socket)
    );

    this.server = server;

    await new Promise<void>((resolve) => {
      server.listen(resolve);
    });

    this.spawnWorkers();
    this.updateInterval = setInterval(() => {
      this.tick();
    }, TestServer.UPDATE_INTERVAL_MS);
  }

  handleSocketConnected(socket: net.Socket) {
    socket.on("data", (data: Buffer) => {
      let result;
      try {
        result = this.handleData(socket, data);
      } catch (e: any) {
        this.handleError(TestServer.ERROR_CODE_HANDLE_DATA, e);
        return;
      }
      const { response, workerId } = result;
      if (workerId === undefined) {
        throw new Error("Could not determine workerId from socket or message");
      }
      const workerData = this.workerData[workerId];
      workerData.writeInProgress = true;
      socket.write(JSON.stringify(response), (err) => {
        workerData.writeInProgress = false;
        if (err) {
          return this.handleError(
            TestServer.ERROR_CODE_SOCKET_WRITE_ERROR,
            err,
            {
              socket,
            }
          );
        }
        if (this.shutdownPending) {
          this.shutdown();
        }
      });
    });
    socket.on("error", (error) => {
      const workerId = this.workerIdBySocket.get(socket);
      if (!workerId || !this.workerData[workerId].completed) {
        if (workerId) {
          const workerData = this.workerData[workerId];
          const stdErr = workerData.stdErrBuffer.getContent().toString();
          const stdOut = workerData.stdOutBuffer.getContent().toString();
          let errorOutput = Color.RED_BACKGROUND(Color.BOLD("Worker Error:\n"));

          if (stdErr) {
            errorOutput += Color.RED(`\nStd Err:\n${stdErr}`);
          }
          if (stdOut) {
            errorOutput += Color.RED(`\nStd Out:\n${stdOut}`);
          }
          console.error(errorOutput);
        }
        this.handleError(TestServer.ERROR_CODE_SOCKET_ERROR, error, {
          socket,
        });
      }
    });
  }

  spawnWorkers() {
    while (this.activeWorkers < this.workerCount && this.fileQueue.length > 0) {
      const file = this.fileQueue.shift()!;
      const workerId = this.nextWorkerId++;

      this.workerData[workerId] = {
        writeInProgress: false,
        currentTest: null,
        success: true,
        completed: false,
        result: null,
        file: file,
        currentTimeout: TestServer.DEFAULT_TIMEOUT_MS,
        lastUpdate: Date.now(),
        currentPath: [],
        connectionTimeout: null,
        stdOutBuffer: new CircularBuffer(1024, 64),
        stdErrBuffer: new CircularBuffer(1024, 64),
      };

      this.results.set(file, {
        results: [],
        name: path.basename(file),
        success: true,
        started: 0,
        ended: 0,
        printed: false,
      });

      this.workerDataFileInProgress.set(file, this.workerData[workerId]);
      this.spawnWorker(workerId, file);
      this.activeWorkers++;
    }
  }

  private spawnWorker(id: number, file: string) {
    const workerData = this.workerData[id];
    const stdOutBuffer = workerData.stdOutBuffer;
    const stdErrBuffer = workerData.stdErrBuffer;

    let env: any = {
      ...process.env,
      __LLRT_TEST_SERVER_PORT: (this.server?.address() as any).port,
      __LLRT_TEST_WORKER_ID: id.toString(),
      __LLRT_TEST_FILE: file,
    };
    delete env.LLRT_LOG;
    const proc = spawn(
      process.argv0,
      ["-e", `import("llrt:test/worker").catch(console.error)`],
      {
        env,
      }
    );
    proc.stderr.on("data", (data) => {
      stdErrBuffer.append(data);
    });
    proc.stdout.on("data", (data) => {
      stdOutBuffer.append(data);
    });
    proc.on("error", (error) => {
      this.handleError(TestServer.ERROR_CODE_PROCESS_ERROR, error, {
        id,
        ended: performance.now(),
      });
    });
    proc.on("exit", (code) => {
      if (code != 0) {
        this.handleError(
          TestServer.ERROR_CODE_PROCESS_ERROR,
          new Error("Worker process exited with a non-zero exit code"),
          {
            id,
            ended: performance.now(),
          }
        );
        this.handleWorkerCompleted(id);
      }
    });
    workerData.connectionTimeout = setTimeout(() => {
      try {
        proc.kill();
      } catch {}
    }, 5000);
    workerData.childProc = proc;
  }

  handleError(code: number, error: Error, details?: any) {
    switch (code) {
      case TestServer.ERROR_CODE_HANDLE_DATA: {
        console.error(`Error handling data,`, error);
        process.exit(1);
      }
      case TestServer.ERROR_CODE_SOCKET_WRITE_ERROR:
      case TestServer.ERROR_CODE_SOCKET_ERROR: {
        console.error(`Socket error,`, error);
        process.exit(1);
      }
      case TestServer.ERROR_CODE_PROCESS_ERROR: {
        const { id: workerId, ended } = details;
        this.handleTestError(workerId, error, ended);
        break;
      }
    }
  }

  handleData(
    socket: net.Socket,
    data: Buffer
  ): { response: object | null; workerId: number } {
    const message = JSON.parse(data as any) as SocketReqMsg;
    const { type } = message;

    let workerId = this.workerIdBySocket.get(socket);
    if (workerId === undefined && "workerId" in message) {
      workerId = (message as any).workerId;
    }

    if (workerId !== undefined) {
      this.workerData[workerId].lastUpdate = Date.now();
    }

    switch (type) {
      case "ready": {
        workerId = message.workerId;
        this.workerIdBySocket.set(socket, workerId);
        clearTimeout(this.workerData[workerId].connectionTimeout!);
        break;
      }
      case "module": {
        const { testCount, skipCount, onlyCount } = message;
        this.totalTests += testCount;
        this.totalSkipped += skipCount;
        this.totalOnly += onlyCount;
        break;
      }
      case "start": {
        const { desc: describe, isSuite, started, timeout } = message;
        const workerData = this.workerData[workerId];

        workerData.currentTimeout = timeout || TestServer.DEFAULT_TIMEOUT_MS;

        if (isSuite) {
          const result: SuiteResult = {
            desc: describe,
            tests: [],
            success: true,
            children: [],
            parent: workerData.result,
            started: 0,
            ended: 0,
          };
          if (!result.parent) {
            const suite = this.results.get(workerData.file!)!;
            suite.started = started;
            suite.results.push(result);
          } else {
            workerData.result!.children.push(result);
          }
          workerData.result = result;
        } else {
          const test: TestResult = {
            desc: describe,
            success: true,
            started,
            ended: 0,
            error: null,
          };
          workerData.result!.tests.push(test);
          workerData.currentTest = test;
        }
        workerData.currentPath.push(describe);

        break;
      }
      case "end": {
        const { isSuite, ended, started } = message;
        const workerData = this.workerData[workerId]!;
        const currentResult = workerData.result!;
        //if we're not in a test
        workerData.lastUpdate = 0;
        if (isSuite) {
          currentResult.ended = ended;
          currentResult.started = started;
          workerData.result = currentResult.parent;
          if (!workerData.result) {
            const suite = this.results.get(workerData.file!)!;
            suite.ended = ended;
            suite.started = started;
            if (workerData.success) {
              this.filesCompleted.add(workerData.file!);
            }
          }
        } else {
          this.totalSuccess++;
          const test = workerData.currentTest!;
          test.ended = ended;
          test.success = true;
        }

        workerData.currentPath.pop();

        break;
      }
      case "error": {
        const { error, ended, workerId: msgWorkerId } = message;
        const effectiveWorkerId = workerId ?? msgWorkerId;
        if (effectiveWorkerId !== undefined) {
          this.handleTestError(effectiveWorkerId, error, ended);
        } else {
          console.error("Error from unknown worker:", error);
        }
        break;
      }
      case "completed": {
        this.handleWorkerCompleted(workerId!);
        break;
      }
      default:
        throw new Error("Unknown type");
    }
    return { response: null, workerId: workerId! };
  }
  private handleWorkerCompleted(workerId: number) {
    const workerData = this.workerData[workerId];
    if (workerData.completed) {
      return;
    }
    workerData.completed = true;
    this.completedWorkers++;
    this.activeWorkers--;

    const shutdownOrPrint = () => {
      if (
        this.completedWorkers == this.testFiles.length &&
        !this.printFinalResults
      ) {
        this.printFinalResults = true;
        clearInterval(this.updateInterval!);
        this.tick();
        this.printResults();
        if (!workerData.writeInProgress) {
          this.shutdown();
        } else {
          this.shutdownPending = true;
        }
      }
    };

    if (workerData.childProc) {
      const exitTimeout = setTimeout(() => {
        const error = new Error(
          "Test did not exit within 1s. It does not properly clean up created resources (servers, timeouts etc)"
        );
        this.handleTestError(workerId, error, performance.now());
        try {
          workerData.childProc?.kill();
        } catch {}
      }, 1000);

      workerData.childProc?.once("exit", () => {
        clearTimeout(exitTimeout);
        shutdownOrPrint();
      });
    } else {
      shutdownOrPrint();
    }
    this.spawnWorkers();
  }

  shutdown() {
    this.shutdownPending = false;
    this.server?.close(() => {
      //XXX force exit on windows
      if (IS_WINDOWS) {
        process.exit(0);
      }
    });
  }
  handleTestError(workerId: number, error: any, ended: number) {
    const workerData = this.workerData[workerId];
    const test = workerData.currentTest || {
      desc: "",
      success: false,
      started: 0,
      ended: 0,
      error,
    };
    workerData.success = false;
    const results = this.results.get(workerData.file!);
    if (results) {
      results.success = false;
    }
    const testFailures = this.filesFailed.get(workerData.file!) || [];
    testFailures.push({
      desc: workerData.currentPath.slice(1),
      error,
      stdErr: workerData.stdErrBuffer.getContent().toString(),
      stdOut: workerData.stdOutBuffer.getContent().toString(),
    });
    workerData.stdErrBuffer.clear();
    workerData.stdOutBuffer.clear();
    this.filesFailed.set(workerData.file!, testFailures);
    this.totalFailed++;
    test.ended = ended;
    test.error = error;
    test.success = false;
    workerData.currentPath.pop();
  }

  private tick() {
    const now = Date.now();
    const first = this.lastUpdate == 0;
    if (now - this.lastUpdate > TestServer.UPDATE_INTERVAL_MS) {
      this.spinnerFrameIndex =
        (this.spinnerFrameIndex + 1) % TestServer.SPINNER.length;
      this.lastUpdate = now;
    }

    //check for hanged tests
    for (let id in this.workerData) {
      const workerData = this.workerData[id];
      if (
        !workerData.completed &&
        workerData.lastUpdate > 0 &&
        now - workerData.lastUpdate >= workerData.currentTimeout
      ) {
        this.handleTestError(
          id as any,
          new Error(`Test timed out after ${workerData.currentTimeout}ms`),
          performance.now()
        );
        try {
          workerData.childProc?.kill();
        } catch {}
        workerData.childProc = undefined;
        this.handleWorkerCompleted(parseInt(id));
      }
    }

    let [terminalWidth] = dimensions();
    let message = "";

    if (!first) {
      //clear last line
      message = "\x1b[F\x1b[2K";
    }

    const spinnerFrame = TestServer.SPINNER[this.spinnerFrameIndex];

    if (terminalWidth > 80) {
      terminalWidth = 80;
    }

    const total = this.testFiles.length;
    const progress = (this.filesCompleted.size + this.filesFailed.size) / total;

    const progressText = `${this.totalSuccess}/${this.totalTests}`;

    const progressbarWidth = Math.min(
      TestServer.DEFAULT_PROGRESS_BAR_WIDTH,
      Math.max(
        10,
        terminalWidth - (2 + progressText.length + 2) //[ + ] + spinner + spacing + progress text
      )
    );
    let totalProgressBarWidth = progressbarWidth;
    const showProgressBarDesc =
      totalProgressBarWidth == TestServer.DEFAULT_PROGRESS_BAR_WIDTH;
    if (showProgressBarDesc) {
      totalProgressBarWidth += TestServer.TESTING_TEXT.length;
    }
    let isSuccess = false;
    let isFailed = false;
    let i = 0;
    let suffix = "";
    let overflow = false;

    for (let file of this.testFiles) {
      let results = this.results.get(file);
      isSuccess = this.filesCompleted.has(file);
      if (!isSuccess) {
        isFailed = this.filesFailed.has(file);
      }
      if (results && (isSuccess || isFailed)) {
        if (!results.printed) {
          results.printed = true;
          message += isSuccess
            ? Color.GREEN(TestServer.CHECKMARK)
            : Color.RED(TestServer.CROSS);
          message += " ";
          message += results.name;
          message += "\n";
        }
        i++;
        continue;
      }

      const inProgress = this.workerDataFileInProgress.has(file);
      const filename = this.testFileNames[i];

      if (
        inProgress &&
        totalProgressBarWidth + suffix.length + 4 < terminalWidth
      ) {
        if (
          totalProgressBarWidth + suffix.length + filename.length + 4 <
          terminalWidth
        ) {
          suffix += filename;
          suffix += ", ";
        } else {
          overflow = true;
          suffix += filename.slice(
            0,
            terminalWidth - (totalProgressBarWidth + suffix.length + 5)
          );
          suffix += "...";
        }
      }

      i++;
    }

    if (!overflow) {
      suffix = suffix.slice(0, -2);
    }
    const elapsed = Math.floor(progressbarWidth * progress);
    const remaining = progressbarWidth - elapsed;

    message += spinnerFrame;
    if (showProgressBarDesc) {
      message += Color.CYAN_BOLD(TestServer.TESTING_TEXT);
    }
    message += `[${"=".repeat(elapsed)}${"-".repeat(remaining)}]`;
    message += progressText;
    message += ": ";
    message += Color.DIM(suffix);

    console.log(message);
  }
  private printResults() {
    const ended = performance.now();
    let output = "";
    for (let file of this.testFiles) {
      const suite = this.results.get(file)!;

      output += `${
        suite.success
          ? Color.GREEN_BACKGROUND(Color.BOLD(" PASS "))
          : Color.RED_BACKGROUND(Color.BOLD(" FAIL "))
      } ${suite.name} ${Color.DIM(TestServer.elapsed(suite))}\n`;

      for (let result of suite.results) {
        output += this.printSuiteResult(result);
      }
      output += "\n";
    }

    if (this.totalFailed == 0) {
      output += Color.GREEN_BACKGROUND(
        Color.BOLD(` ${TestServer.CHECKMARK} ALL PASS `)
      );
    } else {
      output += Color.RED_BACKGROUND(
        Color.BOLD(` ${TestServer.CHECKMARK} TESTS FAILED `)
      );
    }
    output += ` ${Color.DIM(TestServer.elapsed({ started: this.started, ended }))}\n`;
    output += `${this.totalSuccess} passed, ${this.totalFailed} failed, ${this.totalSkipped} skipped, ${this.totalTests} tests\n`;
    console.log(output);
    if (this.totalFailed > 0) {
      output = "";
      const sortedFilesFailed = new Map(
        Array.from(this.filesFailed.entries()).sort(([keyA], [keyB]) =>
          keyA.localeCompare(keyB)
        )
      );
      for (let [file, testFailure] of sortedFilesFailed) {
        output += `\n${Color.RED_BACKGROUND(` ${file} `)}\n`;

        for (let failure of testFailure) {
          output +=
            failure.desc.map((d) => Color.BOLD(d)).join(" > ") +
            `\n${this.formattedError(failure.error)}\n`;
          if (failure.stdOut) {
            output += "----- LAST STDOUT: -----\n" + failure.stdOut + "\n";
          }
          if (failure.stdErr) {
            output += "----- LAST STDERR: -----\n" + failure.stdErr + "\n";
          }
        }
      }
      process.exitCode = 1;
      console.error(output);
    }
  }

  private printSuiteResult(result: SuiteResult, depth = 0): string {
    let output = "";
    const indent = "  ".repeat(depth);
    for (let test of result.tests) {
      const icon = test.success
        ? Color.GREEN(TestServer.CHECKMARK)
        : Color.RED(TestServer.CROSS);
      output += `${indent}${icon} ${test.desc} ${Color.DIM(TestServer.elapsed(test))}\n`;
      if (test.error) {
        output += this.formattedError(test.error) + "\n";
      }
    }
    const results = result.children;
    for (let result of results) {
      output += `${indent}${Color.BOLD(result.desc)} ${Color.DIM(TestServer.elapsed(result))}\n`;
      output += this.printSuiteResult(result, depth + 1);
    }
    return output;
  }
  private formattedError(error: Error, indent: string = ""): string {
    let stack = error.stack || "";

    if (indent && stack) {
      stack = stack
        .split("\n")
        .map((line) => indent + line)
        .join("\n");
    }

    return Color.RED(
      `${indent}\x1b[1m${error.name}:\x1b[22m ${error.message}\n${stack}`
    );
  }

  static elapsed({
    started,
    ended,
  }: {
    started: number;
    ended: number;
  }): string {
    return `${(ended - started).toFixed(3)}ms`;
  }
}

const testServer = new TestServer((globalThis as any).__testEntries, {
  workerCount: 1,
});
await testServer.start();
