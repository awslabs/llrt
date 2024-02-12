// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
global.ReadableStream = class ReadableStream {
  constructor() {
    throw new Error(
      `ReadableStream is not supported via global scope. Enable this by adding this to your code:\n\timport { ReadableStream } from "stream";\n\tglobalThis.ReadableStream = ReadableStream;`
    );
  }
};

__bootstrap.initTasks = [];
const initTasks = __bootstrap.initTasks;
__bootstrap.addInitTask = (task: Promise<any>) => {
  initTasks.push(task);
};

__bootstrap.invokeHandler = async (event: any) =>
  global.__handler(event).then(JSON.stringify);

const REGION = process.env.AWS_REGION || "us-east-1";

const INITED = new Set<string>();

__bootstrap.addAwsSdkInitTask = (service: string) => {
  const prefix = `${service}.${REGION}`;
  if (INITED.has(prefix)) {
    return;
  }
  INITED.add(prefix);
  const start = Date.now();
  const connectTask = fetch(`https://${prefix}.amazonaws.com`, {
    method: "GET",
  }).then(() => {
    if (process.env.LLRT_LOG) {
      console.log("INIT_CONNECTION", service, `${Date.now() - start}ms`);
    }
  });
  initTasks.push(connectTask);
};
