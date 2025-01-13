// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
import net from "net";
import os from "os";
import { spawn, ChildProcess } from "child_process";
import path from "path";
import { SocketReqMsg } from "./shared";
import { platform } from "os";
const IS_WINDOWS = platform() === "win32";
import CircularBuffer from "./CircularBuffer";

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
  stdOutBuffer: CircularBuffer;
  stdErrBuffer: CircularBuffer;
  completed: boolean;
  childProc?: ChildProcess;
  lastUpdate: number;
  success: boolean;
  connectionTimeout: Timeout | null;
  currentTest: TestResult | null;
  currentResult: SuiteResult | null;
  currentFile: string | null;
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

    await new Promise((resolve) => {
      server.listen(resolve);
    });

    this.spawnAllWorkers();
    this.updateInterval = setInterval(() => {
      this.tick();
    }, TestServer.UPDATE_INTERVAL_MS);
  }

  handleSocketConnected(socket: net.Socket) {
    socket.on("data", (data) => {
      let response;
      try {
        response = this.handleData(socket, data);
      } catch (e: any) {
        this.handleError(TestServer.ERROR_CODE_HANDLE_DATA, e);
        return;
      }
      socket.write(JSON.stringify(response), (err) => {
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
        this.handleError(TestServer.ERROR_CODE_SOCKET_ERROR, error, {
          socket,
        });
      }
    });
  }

  spawnAllWorkers() {
    for (let i = 0; i < this.workerCount; i++) {
      this.workerData[i] = {
        currentTest: null,
        success: true,
        completed: false,
        currentResult: null,
        currentFile: null,
        currentTimeout: TestServer.DEFAULT_TIMEOUT_MS,
        lastUpdate: Date.now(),
        currentPath: [],
        connectionTimeout: null,
        stdOutBuffer: new CircularBuffer(1024, 64),
        stdErrBuffer: new CircularBuffer(1024, 64),
      };
      this.spawnWorker(i);
    }
  }

  private spawnWorker(id: number) {
    const workerData = this.workerData[id];
    const stdOutBuffer = workerData.stdOutBuffer;
    const stdErrBuffer = workerData.stdErrBuffer;

    let env: any = {
      ...process.env,
      __LLRT_TEST_SERVER_PORT: (this.server?.address() as any).port,
      __LLRT_TEST_WORKER_ID: id.toString(),
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
        this.handleWorkerCompleted(id, true);
      }
    });
    workerData.connectionTimeout = setTimeout(() => {
      proc.kill();
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

  handleData(socket: net.Socket, data: Buffer): object | null {
    const message = JSON.parse(data as any) as SocketReqMsg;
    const { type } = message;

    const workerId = this.workerIdBySocket.get(socket)!;

    if (workerId) {
      this.workerData[workerId].lastUpdate = Date.now();
    }

    switch (type) {
      case "ready": {
        let { workerId } = message;
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
      case "next": {
        const nextFile = this.fileQueue.shift();
        const workerData = this.workerData[workerId];
        workerData.stdErrBuffer.clear();
        workerData.stdOutBuffer.clear();
        //clear current path

        workerData.currentPath.length = 0;

        if (nextFile) {
          this.results.set(nextFile, {
            results: [],
            name: path.basename(nextFile),
            success: true,
            started: 0,
            ended: 0,
            printed: false,
          });
          workerData.currentFile = nextFile;
          this.workerDataFileInProgress.set(nextFile, workerData);
        } else {
          workerData.currentFile = null;
          workerData.lastUpdate = 0;
        }
        return { nextFile: nextFile || null };
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
            parent: workerData.currentResult,
            started: 0,
            ended: 0,
          };
          if (!result.parent) {
            const suite = this.results.get(workerData.currentFile!)!;
            suite.started = started;
            suite.results.push(result);
          } else {
            workerData.currentResult!.children.push(result);
          }
          workerData.currentResult = result;
        } else {
          const test: TestResult = {
            desc: describe,
            success: true,
            started,
            ended: 0,
            error: null,
          };
          workerData.currentResult!.tests.push(test);
          workerData.currentTest = test;
        }
        workerData.currentPath.push(describe);

        break;
      }
      case "end": {
        const { isSuite, ended, started } = message;
        const workerData = this.workerData[workerId]!;
        const currentResult = workerData.currentResult!;
        //if we're not in a test
        workerData.lastUpdate = 0;
        if (isSuite) {
          currentResult.ended = ended;
          currentResult.started = started;
          workerData.currentResult = currentResult.parent;
          if (!workerData.currentResult) {
            const suite = this.results.get(workerData.currentFile!)!;
            suite.ended = ended;
            suite.started = started;
            if (workerData.success) {
              this.filesCompleted.add(workerData.currentFile!);
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
        const { error, ended } = message;
        this.handleTestError(workerId, error, ended);
        break;
      }
      case "completed": {
        this.handleWorkerCompleted(workerId, false);
        break;
      }
      default:
        throw new Error("Unknown type");
    }
    return null;
  }
  private handleWorkerCompleted(workerId: number, shutdownOnComplete: boolean) {
    this.workerData[workerId].completed = true;
    this.completedWorkers++;

    if (this.completedWorkers == this.workerCount) {
      clearInterval(this.updateInterval!);
      this.tick();
      this.printResults();
      if (shutdownOnComplete) {
        this.shutdown();
      } else {
        this.shutdownPending = true;
      }
    }
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
    const results = this.results.get(workerData.currentFile!);
    if (results) {
      results.success = false;
    }
    const testFailures = this.filesFailed.get(workerData.currentFile!) || [];
    testFailures.push({
      desc: workerData.currentPath.slice(1),
      error,
      stdErr: workerData.stdErrBuffer.getContent().toString(),
      stdOut: workerData.stdOutBuffer.getContent().toString(),
    });
    workerData.stdErrBuffer.clear();
    workerData.stdOutBuffer.clear();
    this.filesFailed.set(workerData.currentFile!, testFailures);
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
        workerData.childProc?.kill();
        this.handleWorkerCompleted(parseInt(id), true);
      }
    }

    if (this.completedWorkers != this.workerCount) {
      let [terminalWidth] = (console as any).__dimensions;
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
      const progress =
        (this.filesCompleted.size + this.filesFailed.size) / total;

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

    if (this.totalFailed > 0) {
      for (let [file, testFailure] of this.filesFailed) {
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
      console.log(output);
      process.exit(1);
    }
    console.log(output);
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
  workerCount: undefined,
});
await testServer.start();
