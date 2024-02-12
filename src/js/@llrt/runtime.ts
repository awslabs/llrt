// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
// @ts-ignore
const global = globalThis as any;

type Context = {
  awsRequestId: string;
  invokedFunctionArn: string;
  logGroupName: string;
  logStreamName: string;
  functionName: string;
  functionVersion: string;
  identity?: any;
  clientContext?: any;
  memoryLimitInMB: string;
  callbackWaitsForEmptyEventLoop: boolean;
  getRemainingTimeInMillis: () => number;
};

const {
  AWS_LAMBDA_FUNCTION_NAME,
  AWS_LAMBDA_FUNCTION_VERSION,
  AWS_LAMBDA_FUNCTION_MEMORY_SIZE,
  AWS_LAMBDA_LOG_GROUP_NAME,
  AWS_LAMBDA_LOG_STREAM_NAME,
  LAMBDA_TASK_ROOT,
  _HANDLER,
  LAMBDA_HANDLER,
  AWS_LAMBDA_RUNTIME_API,
  _EXIT_ITERATIONS,
  AWS_REGION,
} = process.env;

if (!AWS_LAMBDA_RUNTIME_API) {
  throw new Error(
    'Environment variable "AWS_LAMBDA_RUNTIME_API" is not defined'
  );
}

const HANDLER_ENV = _HANDLER || LAMBDA_HANDLER;
let requestId: string | undefined;

let exitIterations = (_EXIT_ITERATIONS && parseInt(_EXIT_ITERATIONS)) || -1;
if (isNaN(exitIterations)) {
  exitIterations = -1;
}

const RUNTIME_HEADERS = {
  TRACE_ID: "lambda-runtime-trace-id",
  DEADLINE_MS: "lambda-runtime-deadline-ms",
  REQUEST_ID: "lambda-runtime-aws-request-id",
  INVOKED_FUNCTION_ARN: "lambda-runtime-invoked-function-arn",
  CLIENT_CONTEXT: "lambda-runtime-client-context",
  COGNITO_IDENTITY: "lambda-runtime-cognito-identity",
};

const RUNTIME_PATH = "2018-06-01/runtime";
const HEADERS = {
  "Content-Type": "application/json",
};
const BASE_URL = `http://${AWS_LAMBDA_RUNTIME_API}/${RUNTIME_PATH}`;

const postError = async (path: string, error: any, requestId?: string) => {
  const { name: errorName, message, stack, cause } = error;
  const lambdaError: any = {
    errorType: errorName || typeof error,
    errorMessage: message || "" + error,
    stackTrace: (stack || "").split("\n").slice(0, 20),
  };
  if (requestId) {
    lambdaError.requestId = requestId;
  }
  if (cause) {
    lambdaError.cause = cause;
  }

  const errorBody = JSON.stringify(lambdaError);
  console.error(lambdaError);
  const res = await fetch(`${BASE_URL}${path}`, {
    method: "POST",
    headers: {
      ...HEADERS,
      "Lambda-Runtime-Function-Error-Type": lambdaError.errorType,
    } as any,
    body: errorBody,
  });
  await res.text();
  if (res.status !== 202) {
    throw new Error(
      `Unexpected /${path} response: ${JSON.stringify(res)}\n${res.text()}`
    );
  }
};

const initError = (error: any) => postError(`/init/error`, error);
const invokeError = (error: any, context: Context) =>
  postError(
    `/invocation/${context.awsRequestId}/error`,
    error,
    context.awsRequestId
  );

const nextInvocation = async () => {
  const res = await fetch(`${BASE_URL}/invocation/next`, {
    headers: HEADERS,
  });

  if (res.status !== 200) {
    throw new Error(
      `Unexpected /invocation/next response: ${JSON.stringify(
        res
      )}\n${res.text()}`
    );
  }

  const traceIdValue = res.headers.get(RUNTIME_HEADERS.TRACE_ID);
  if (traceIdValue) {
    process.env._X_AMZN_TRACE_ID = traceIdValue;
  } else {
    delete process.env._X_AMZN_TRACE_ID;
  }

  const deadlineMs = +res.headers.get(RUNTIME_HEADERS.DEADLINE_MS)!;
  const clientContextJson = res.headers.get(RUNTIME_HEADERS.CLIENT_CONTEXT);
  const cognitoIdentityJson = res.headers.get(RUNTIME_HEADERS.COGNITO_IDENTITY);

  let context: Context = {
    awsRequestId: res.headers.get(RUNTIME_HEADERS.REQUEST_ID)!,
    invokedFunctionArn: res.headers.get(RUNTIME_HEADERS.INVOKED_FUNCTION_ARN)!,
    logGroupName: AWS_LAMBDA_LOG_GROUP_NAME!,
    logStreamName: AWS_LAMBDA_LOG_STREAM_NAME!,
    functionName: AWS_LAMBDA_FUNCTION_NAME!,
    functionVersion: AWS_LAMBDA_FUNCTION_VERSION!,
    memoryLimitInMB: AWS_LAMBDA_FUNCTION_MEMORY_SIZE!,
    getRemainingTimeInMillis: () => deadlineMs - Date.now(),
    callbackWaitsForEmptyEventLoop: true,
    clientContext:
      (clientContextJson && JSON.parse(clientContextJson)) || undefined,
    identity:
      (cognitoIdentityJson && JSON.parse(cognitoIdentityJson)) || undefined,
  };

  const event = await res.json();
  requestId = context.awsRequestId;

  return { event, context };
};

const invokeResponse = async (result: any, context: Context) => {
  const res = await fetch(
    `${BASE_URL}/invocation/${context.awsRequestId}/response`,
    {
      method: "POST",
      body: JSON.stringify(result === undefined ? null : result) as any,
      headers: HEADERS,
    }
  );
  if (res.status !== 202) {
    throw new Error(
      `Unexpected /invocation/response response: ${JSON.stringify(
        res
      )}\n${res.text()}`
    );
  }
};

let iterations = 0;
const startProcessEvents = async (
  handler: (event: any, context: Context) => Promise<any>
) => {
  let context = null;
  let event = null;
  while (true) {
    const start = new Date().getTime();
    try {
      const next = await nextInvocation();
      __bootstrap.setRequestId(requestId);
      context = next.context;
      event = next.event;
      const result = await handler(event, context);
      await invokeResponse(result, context);
    } catch (e: any) {
      console.error(e["stack"]);
      if (!context) {
        console.error("error: failed to get next response", e);
        process.exit(1);
      }
      try {
        await invokeError(e, context!);
      } catch (e2) {
        console.error("error: failed to run invoke error", e2);
        process.exit(1);
      }
    }

    if (exitIterations > -1) {
      if (iterations >= exitIterations - 1) {
        console.log(`Done in ${new Date().getTime() - start}ms`);
        break;
      }
      iterations++;
    }
  }
};

const main = async () => {
  try {
    const [moduleName, handlerName] = HANDLER_ENV!.split(".") || [null, null];

    if (moduleName == null || handlerName == null) {
      throw new Error(
        "Invalid handler name or LAMBDA_HANDLER env: Should be in format {filename}.{method_name}"
      );
    }

    const taskRoot = LAMBDA_TASK_ROOT || process.cwd();
    const handlerModule = await import(`${taskRoot}/${moduleName}`);
    const { init } = handlerModule;

    const initTasks = __bootstrap.initTasks;

    if (init && typeof init === "function") {
      initTasks.push(init());
    }

    if (initTasks.length === 1) {
      await initTasks[0];
    } else if (initTasks.length > 1) {
      await Promise.all(initTasks);
    }
    const handler = handlerModule[handlerName];

    if (typeof handler !== "function") {
      throw new Error(`"${handlerName}" is not a function in "${moduleName}"`);
    }
    await startProcessEvents(handler);
  } catch (error) {
    console.error(error);
    try {
      await initError(error);
    } catch (nestedError) {
      console.error("error: failed to run init error", nestedError);
    }
    process.exit(1);
  }
};

main();
