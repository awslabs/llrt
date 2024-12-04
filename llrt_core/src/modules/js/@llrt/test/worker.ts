import net from "net";
import * as chai from "chai";
import { JestChaiExpect } from "../expect/jest-expect";
import { JestAsymmetricMatchers } from "../expect/jest-asymmetric-matchers";
import { SocketReqMsg, SocketResponseMap } from "./shared";
import SocketClient from "./SocketClient";

type Test = TestSettings & {
  desc: string;
  fn: (done?: (error?: any) => void) => Promise<void>;
};

type TestSettings = {
  only?: boolean;
  skip?: boolean;
  timeout?: number;
};

type SuiteFunctionWithOptions = SuiteFunction & {
  skip?: SuiteFunction;
  only?: SuiteFunction;
};

type SuiteFunction = (
  desc: string,
  fn: () => Promise<void>,
  timeout?: number
) => void;

type TestSuite = TestSettings &
  TestSetup & {
    tests?: Test[];
    suites?: TestSuite[];
    parent?: TestSuite;
    containsOnly?: boolean;
    desc: string;
  };

type TestSetup = {
  afterAll?: MaybeAsyncFunction;
  afterEach?: MaybeAsyncFunction;
  beforeAll?: MaybeAsyncFunction;
  beforeEach?: MaybeAsyncFunction;
};

type RootSuite = TestSettings &
  TestSetup &
  Required<Omit<TestSuite, "parent" | keyof TestSettings | keyof TestSetup>> & {
    onlyCount: number;
    testCount: number;
    skipCount: number;
    module: string;
  };

type MaybeAsyncFunction = () => Promise<void> | void;

type MessageTypeMap = {
  [K in SocketReqMsg["type"]]: Extract<SocketReqMsg, { type: K }>;
};

type MessagePayload<T extends SocketReqMsg["type"]> = Omit<
  MessageTypeMap[T],
  "type"
>;

type SocketReturnType<T> = T extends keyof SocketResponseMap
  ? SocketResponseMap[T]
  : null;

class TestAgent {
  private static DEFAULT_TIMEOUT_MS =
    parseInt((process.env as any).TEST_TIMEOUT) || 5000;

  private static EMPTY_ARROW_FN_REGEX = /^(async)?\s*\(\s*\)\s*=>/m;
  private static EMPTY_FN_REGEX =
    /^(async)?\s*function\s*[a-zA-Z0-9_-]*\s*\(\s*\)\s*\{/m;

  private static EXPECT = (() => {
    chai.use(JestChaiExpect);
    chai.use(JestAsymmetricMatchers);
    const expect = chai.expect as (value: any, message?: string) => any;
    return Object.assign(expect, chai.expect);
  })();

  private workerId: number;
  private client: SocketClient;
  private rootSuite: RootSuite = TestAgent.createRootSuite();
  private currentSuite!: TestSuite;
  private currentSuites: TestSuite[] = [];
  private suiteLoadPromises: (() => Promise<void>)[] = [];
  private describe: SuiteFunctionWithOptions;
  private testFunction: SuiteFunctionWithOptions;
  private onlyCount: number = 0;

  static createRootSuite(): RootSuite {
    return {
      tests: [],
      suites: [],
      containsOnly: false,
      desc: "root",
      testCount: 0,
      skipCount: 0,
      onlyCount: 0,
      module: "",
    };
  }

  constructor(workerId: number, serverPort: number) {
    this.workerId = workerId;
    this.client = new SocketClient("localhost", serverPort);

    this.client.on("error", (err) => {
      console.error("Worker Client Socket Error:", workerId, err);
    });

    const testFunction = this.createTestFunction();
    testFunction.only = this.createTestFunction({ only: true });
    testFunction.skip = this.createTestFunction({ skip: true });

    const describe = this.createDescribe();
    describe.only = this.createDescribe({ only: true });
    describe.skip = this.createDescribe({ skip: true });

    this.describe = describe;
    this.testFunction = testFunction;
  }

  private createDescribe({
    only = false,
    skip = false,
  }: TestSettings = {}): SuiteFunctionWithOptions {
    return (desc: string, fn: () => Promise<void>, timeout?: number) => {
      this.suiteLoadPromises.push(async () => {
        let parent: TestSuite = this.currentSuites.shift() ?? this.rootSuite;
        this.currentSuite = {
          tests: [],
          suites: [],
          parent,
          only: only || parent.only,
          skip,
          desc,
          timeout: timeout || parent.timeout,
        };
        parent.suites!!.push(this.currentSuite);
        let beforeLength = this.suiteLoadPromises.length;

        await fn();
        let afterLength = this.suiteLoadPromises.length;

        let items = this.suiteLoadPromises.splice(
          beforeLength,
          afterLength - beforeLength
        );
        if (items.length) {
          this.suiteLoadPromises.unshift(...items);
          let subSuites = new Array(items.length).fill(this.currentSuite);
          this.currentSuites.unshift(...subSuites);
        }
      });
    };
  }

  private createTestFunction({
    only = false,
    skip = false,
  }: TestSettings = {}): SuiteFunctionWithOptions {
    return (desc: string, fn: () => Promise<void>, timeout?: number) => {
      let suite: TestSuite = this.currentSuite;
      this.rootSuite.testCount++;
      if (skip || suite?.skip) {
        this.rootSuite.skipCount++;
        return;
      }
      let onlyValue = only || suite.only;
      if (onlyValue) {
        this.onlyCount++;
        suite.containsOnly = true;
        let p = suite.parent;

        while (p) {
          p.containsOnly = true;
          p = p?.parent;
        }
      }

      const test = {
        desc,
        fn,
        only: onlyValue,
        timeout: timeout || suite.timeout,
      };
      suite.tests?.push(test);
    };
  }

  private async executeAsyncOrCallbackFn(
    fn: Function,
    timeout: number = TestAgent.DEFAULT_TIMEOUT_MS
  ) {
    const fnBody = fn.toString();
    const usesArgument = !(
      TestAgent.EMPTY_ARROW_FN_REGEX.test(fnBody) ||
      TestAgent.EMPTY_FN_REGEX.test(fnBody)
    );
    TestAgent.EMPTY_ARROW_FN_REGEX.lastIndex = -1;
    TestAgent.EMPTY_FN_REGEX.lastIndex = -1;

    const timeoutMessage = `Timeout after ${timeout}ms`;

    if (usesArgument) {
      await new Promise<void>((resolve, reject) => {
        const timeoutId = setTimeout(() => reject(timeoutMessage), timeout);
        const resolveWrapper = (error: any) => {
          clearTimeout(timeoutId);
          if (error) {
            return reject(error);
          }
          resolve();
        };
        Promise.resolve(fn(resolveWrapper)).catch(reject);
      });
    } else {
      let timeoutId: Timeout;
      const timeoutPromise = new Promise<void>((_, reject) => {
        timeoutId = setTimeout(() => reject(timeoutMessage), timeout);
      });
      try {
        await Promise.race([Promise.resolve(fn()), timeoutPromise]);
      } catch (e) {
        clearTimeout(timeoutId!);
        throw e;
      }
      clearTimeout(timeoutId!);
    }
  }

  private sendWorkerId() {
    return this.sendMessage("ready", {
      workerId: this.workerId,
    });
  }

  private async nextTestFile() {
    return await this.sendMessage("next");
  }

  private async complete() {
    console.log("BEFORE worker complete:", this.workerId);
    await this.sendMessage("completed");
    console.log("AFTER worker complete", this.workerId);
    await this.client.close();
    console.log("AFTER worker close", this.workerId);
  }

  private async sendMessage<T extends SocketReqMsg["type"]>(
    type: T,
    ...message: MessagePayload<T> extends Record<string, never>
      ? []
      : [MessagePayload<T>]
  ): Promise<SocketReturnType<T>> {
    const [messageData] = message!;

    if (type == "error") {
      const errorData = messageData as MessagePayload<"error">;

      if (typeof errorData.error === "string") {
        errorData.error = {
          message: errorData.error,
          name: "Error",
        };
      } else {
        errorData.error = {
          message: errorData.error.message,
          stack: errorData.error.stack,
          name: errorData.error.name,
        };
      }
    }

    const data = JSON.stringify({
      type,
      ...messageData,
    });
    const response = (await this.client.send(data)) as any;
    return JSON.parse(response) as SocketReturnType<T>;
  }

  private async runTests(testSuite: RootSuite, tests: Test[] = []) {
    for (const test of tests) {
      if (test.skip || (this.onlyCount > 0 && !test.only)) {
        continue;
      }

      let started = performance.now();

      try {
        await this.sendMessage("start", {
          started,
          desc: test.desc,
          isSuite: false,
          timeout: test.timeout,
        });

        if (testSuite.beforeEach) {
          await this.executeAsyncOrCallbackFn(testSuite.beforeEach);
        }

        started = performance.now();

        await this.executeAsyncOrCallbackFn(test.fn, test.timeout);

        const end = performance.now();

        if (testSuite.afterEach) {
          await this.executeAsyncOrCallbackFn(testSuite.afterEach);
        }

        await this.sendMessage("end", {
          ended: end,
          started,
          isSuite: false,
        });
      } catch (error: any) {
        await this.sendMessage("error", {
          error,
          started,
          ended: performance.now(),
        });
      }
    }
  }

  public async start(): Promise<void> {
    await this.connect();
    await this.sendWorkerId();

    const global: any = globalThis;

    while (true) {
      const started = performance.now();
      try {
        const { nextFile: entry } = await this.nextTestFile();
        if (!entry) {
          break;
        }

        const rootSuite = TestAgent.createRootSuite();
        this.rootSuite = rootSuite;
        this.onlyCount = 0;

        this.currentSuite = this.rootSuite;
        this.currentSuites = [];

        let index = entry.lastIndexOf("/");
        if (index !== -1) {
          rootSuite.module = entry.substring(index + 1);
        } else {
          rootSuite.module = entry;
        }

        global.it = this.testFunction;
        global.test = this.testFunction;
        global.describe = this.describe;
        global.expect = TestAgent.EXPECT;

        global.beforeEach = (cb: MaybeAsyncFunction) => {
          this.currentSuite.beforeEach = cb;
        };

        global.beforeAll = (cb: MaybeAsyncFunction) => {
          this.currentSuite.beforeAll = cb;
        };

        global.afterEach = (cb: MaybeAsyncFunction) => {
          this.currentSuite.afterEach = cb;
        };

        global.afterAll = (cb: MaybeAsyncFunction) => {
          this.currentSuite.afterAll = cb;
        };

        await import(entry);

        while (this.suiteLoadPromises.length > 0) {
          const suitePromise = this.suiteLoadPromises.shift()!;
          await suitePromise();
        }

        await this.sendMessage("module", {
          skipCount: rootSuite.skipCount,
          testCount: rootSuite.testCount,
          onlyCount: rootSuite.onlyCount,
        });

        await this.runRootSuite();

        delete global.it;
        delete global.expect;
        delete global.test;
        delete global.describe;
        delete global.beforeEach;
        delete global.beforeAll;
        delete global.afterEach;
        delete global.afterAll;
      } catch (error) {
        try {
          await this.sendMessage("error", {
            error,
            started,
            ended: performance.now(),
          });
        } catch (e) {
          console.error("Error sending error message:", e);
          process.exit(1);
        }
      }
    }

    await this.complete();
  }
  async runRootSuite() {
    const testSuite = this.rootSuite;
    const started = performance.now();

    try {
      await this.sendMessage("start", {
        desc: testSuite.module,
        isSuite: true,
        started,
        timeout: testSuite.timeout,
      });
      if (testSuite.beforeAll) {
        await this.executeAsyncOrCallbackFn(
          testSuite.beforeAll,
          testSuite.timeout
        );
      }
      await this.runTests(testSuite, testSuite.tests);
      const stack = [...testSuite.suites];
      while (stack.length > 0) {
        const suite = stack.shift()!;
        const suiteStarted = performance.now();
        await this.sendMessage("start", {
          desc: suite.desc,
          isSuite: true,
          started: suiteStarted,
          timeout: suite.timeout,
        });
        if (
          suite.skip ||
          (this.onlyCount > 0 && !suite.only && !suite.containsOnly)
        ) {
          continue;
        }

        try {
          if (suite.beforeAll) {
            await this.executeAsyncOrCallbackFn(suite.beforeAll);
          }
          await this.runTests(testSuite, suite.tests);
          if (suite.afterAll) {
            await this.executeAsyncOrCallbackFn(suite.afterAll);
          }
          await this.sendMessage("end", {
            isSuite: true,
            started: suiteStarted,
            ended: performance.now(),
          });
        } catch (error: any) {
          await this.sendMessage("error", {
            error,
            started,
            ended: performance.now(),
          });
        }

        if (suite.suites) {
          stack.unshift(...suite.suites);
        }
      }
      if (testSuite.afterAll) {
        await this.executeAsyncOrCallbackFn(testSuite.afterAll);
      }
      await this.sendMessage("end", {
        isSuite: true,
        started,
        ended: performance.now(),
      });
    } catch (error: any) {
      await this.sendMessage("error", {
        error,
        started,
        ended: performance.now(),
      });
    }
  }
  async connect() {
    await this.client.connect();
  }
}

// Usage example
const {
  __LLRT_TEST_SERVER_PORT: serverPortEnv,
  __LLRT_TEST_WORKER_ID: workerIdEnv,
} = process.env;

const workerId = parseInt(workerIdEnv || "");
const serverPort = parseInt(serverPortEnv || "");

if (isNaN(workerId) || isNaN(serverPort)) {
  throw new Error(
    "Test worker requires __LLRT_TEST_SERVER_PORT & __LLRT_TEST_WORKER_ID env"
  );
}

const agent = new TestAgent(workerId, serverPort);
await agent.start();
