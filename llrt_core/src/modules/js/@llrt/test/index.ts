// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
import net from "net";
import { EventEmitter } from "events";
import os from "os";
import { spawn, ChildProcess } from "child_process";
import path from "path";
import { SocketReqMsg } from "./shared";

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
};

type WorkerData = {
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
  message?: string;
};

class TestServer extends EventEmitter {
  private static UPDATE_FPS = 15;
  private static UPDATE_INTERVAL_MS = 1000 / TestServer.UPDATE_FPS;
  private static DEFAULT_TIMEOUT_MS =
    parseInt((process.env as any).TEST_TIMEOUT) || 5000;

  static SPINNER = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
  static CHECKMARK = "\u2714";
  static CROSS = "\u2718";
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

  constructor(
    testFiles: string[],
    { workerCount = os.availableParallelism() }: TestOptions = {}
  ) {
    super();
    this.fileQueue = [...testFiles];
    this.testFiles = [...testFiles];
    this.testFileNames = testFiles.map((file) => path.basename(file));
    this.filesFailed = new Map();
    this.filesCompleted = new Set();
    this.workerCount = Math.min(workerCount, testFiles.length);
  }

  public async start() {
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
      socket.write(JSON.stringify(response));
    });
    socket.on("error", (error) =>
      this.handleError(TestServer.ERROR_CODE_SOCKET_ERROR, error, {
        socket,
      })
    );
  }

  async spawnAllWorkers() {
    for (let i = 0; i < this.workerCount; i++) {
      this.workerData[i] = {
        currentTest: null,
        success: false,
        completed: false,
        currentResult: null,
        currentFile: null,
        currentTimeout: TestServer.DEFAULT_TIMEOUT_MS,
        lastUpdate: Date.now(),
        currentPath: [],
        connectionTimeout: null,
      };
      this.spawnWorker(i);
    }
  }

  private spawnWorker(id: number) {
    const workerData = this.workerData[id];
    let output = Buffer.from("");
    const proc = spawn(
      process.argv0,
      ["-e", `import("llrt:test/worker").catch(console.error)`],
      {
        env: {
          ...process.env,
          __LLRT_TEST_SERVER_PORT: (this.server?.address() as any).port,
          __LLRT_TEST_WORKER_ID: id.toString(),
        },
      }
    );
    proc.stdout.on("data", (data) => {
      output = data;
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
            output: output.toString(),
          }
        );
        this.handleWorkerCompleted(id);
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
        const { id: workerId, ended, output } = details;
        this.handleTestError(workerId, error, ended, output);
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

        if (nextFile) {
          this.results.set(nextFile, {
            results: [],
            name: path.basename(nextFile),
            success: true,
            started: 0,
            ended: 0,
          });
          workerData.currentFile = nextFile;
          this.workerDataFileInProgress.set(nextFile, workerData);
        } else {
          workerData.currentFile = null;
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
            success: false,
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
        this.handleWorkerCompleted(workerId);

        break;
      }
      default:
        throw new Error("Unknown type");
    }
    return null;
  }
  private handleWorkerCompleted(workerId: number) {
    this.workerData[workerId].completed = true;
    this.completedWorkers++;

    if (this.completedWorkers == this.workerCount) {
      this.tick();
      this.printResults();
      this.shutdown();
    }
  }

  shutdown() {
    clearInterval(this.updateInterval!);
    this.server?.close();
  }
  handleTestError(
    workerId: number,
    error: any,
    ended: number,
    message?: string
  ) {
    const workerData = this.workerData[workerId];
    const test = workerData.currentTest;
    workerData.success = false;
    this.results.get(workerData.currentFile!)!.success = false;

    if (test) {
      const testFailures = this.filesFailed.get(workerData.currentFile!) || [];
      testFailures.push({
        desc: workerData.currentPath.slice(1),
        error,
        message,
      });
      this.filesFailed.set(workerData.currentFile!, testFailures);
      this.totalFailed++;
      test.ended = ended;
      test.error = error;
      test.success = false;
      workerData.currentPath.pop();
    }
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
        now - workerData.lastUpdate >= workerData.currentTimeout
      ) {
        this.handleTestError(
          id as any,
          new Error(`Test timed out after ${workerData.currentTimeout}ms`),
          performance.now()
        );
        workerData.childProc?.kill();
        this.handleWorkerCompleted(parseInt(id));
      }
      // if (workerData.currentTest) {
      //   this.handleTestError(id as any, new Error("Test timed out"), now);
      // }
    }

    let [width, height] = (console as any).__dimensions;
    let message = "";

    if (!first) {
      const lineCount = this.testFiles.length + 1;
      const overflow = lineCount - height;
      if (overflow > 0) {
        message = `\x1b[H\x1b[3J\x1b[J`;
      } else {
        //move to first position of files and clear reminder of screen
        message = `\x1b[${lineCount}F\x1b[J`;
      }
    }

    const spinnerFrame = TestServer.SPINNER[this.spinnerFrameIndex];

    if (height < 10) {
      height = 10;
    }
    if (width < 80) {
      width = 80;
    }

    let isSuccess = false;
    let isFailed = false;
    let i = 0;
    let line;
    let desc;
    for (let file of this.testFiles) {
      line = "";
      isSuccess = this.filesCompleted.has(file);
      if (!isSuccess) {
        isFailed = this.filesFailed.has(file);
      }
      line += isSuccess
        ? Color.GREEN(TestServer.CHECKMARK)
        : isFailed
          ? Color.RED(TestServer.CROSS)
          : spinnerFrame;
      line += ` ${Color.CYAN_BOLD("Testing")} `;
      line += this.testFileNames[i];
      desc = this.workerDataFileInProgress.get(file)?.currentTest?.desc;
      if (!(isSuccess || isFailed) && desc) {
        line += " ";
        line += Color.DIM(desc);
      }
      if (line.length > width) {
        line = line.substring(0, width - 3);
        line += "...";
        line += Color.RESET;
      }

      line += "\n";
      message += line;
      i++;
    }
    const total = this.testFiles.length;
    const progress = (this.filesCompleted.size + this.filesFailed.size) / total;

    const progressText = `${this.totalSuccess}/${this.totalTests}`;
    const availableWidth = width - progressText.length - 2;
    const elapsed = availableWidth * progress;
    const remaining = availableWidth - elapsed;

    message += `[${"=".repeat(elapsed)}${"-".repeat(remaining)}]${progressText}`;
    //console.log(message);
  }
  printResults() {
    const ended = performance.now();
    for (let file of this.testFiles) {
      const suite = this.results.get(file)!;

      console.log(
        suite.success
          ? Color.GREEN_BACKGROUND(Color.BOLD(" PASS "))
          : Color.RED_BACKGROUND(Color.BOLD(" FAIL ")),
        suite.name,
        Color.DIM(TestServer.elapsed(suite))
      );
      for (let result of suite.results) {
        this.printSuiteResult(result);
      }
      console.log("");
    }
    let status = "";
    if (this.totalFailed == 0) {
      status = Color.GREEN_BACKGROUND(
        Color.BOLD(` ${TestServer.CHECKMARK} ALL PASS `)
      );
    } else {
      status = Color.RED_BACKGROUND(
        Color.BOLD(` ${TestServer.CHECKMARK} TESTS FAILED `)
      );
    }
    console.log(
      status,
      Color.DIM(TestServer.elapsed({ started: this.started, ended }))
    );
    console.log(
      `${this.totalSuccess} passed, ${this.totalFailed} failed, ${this.totalSkipped} skipped, ${this.totalTests} tests`
    );
    if (this.totalFailed > 0) {
      for (let [file, testFailure] of this.filesFailed) {
        console.log(`\n${Color.RED_BACKGROUND(` ${file} `)}`);
        for (let failure of testFailure) {
          console.log(
            failure.desc.map((d) => Color.BOLD(d)).join(" > "),
            `\n${this.formattedError(failure.error)}`
          );
          if (failure.message) {
            console.log("----- LAST OUTPUT: -----\n" + failure.message);
          }
        }
      }
      process.exit(1);
    }
  }
  printSuiteResult(result: SuiteResult, depth = 0) {
    const indent = "  ".repeat(depth);
    for (let test of result.tests) {
      const icon = test.success
        ? Color.GREEN(TestServer.CHECKMARK)
        : Color.RED(TestServer.CROSS);
      console.log(
        `${indent}${icon} ${test.desc}`,
        Color.DIM(TestServer.elapsed(test))
      );
      if (test.error) {
        console.log(this.formattedError(test.error));
      }
    }
    const results = result.children;
    for (let result of results) {
      console.log(
        `${indent}${Color.BOLD(result.desc)}`,
        Color.DIM(TestServer.elapsed(result))
      );
      this.printSuiteResult(result, depth + 1);
    }
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
