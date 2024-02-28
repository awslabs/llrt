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

const REGION = process.env.AWS_REGION || "us-east-1";
const IS_LAMBDA =
  !!process.env.AWS_LAMBDA_RUNTIME_API && !!process.env._HANDLER;
const INITED = new Set<string>();

__bootstrap.addAwsSdkInitTask = (service: string) => {
  if (IS_LAMBDA) {
    const prefix = `${service}.${REGION}`;
    if (INITED.has(prefix)) {
      return;
    }
    INITED.add(prefix);
    const start = Date.now();
    const connectTask = fetch(`https://${prefix}.amazonaws.com`, {
      method: "GET",
    }).then((res) => {
      const _ = res.arrayBuffer(); //take the response
      if (process.env.LLRT_LOG) {
        console.log("INIT_CONNECTION", service, `${Date.now() - start}ms`);
      }
    });
    initTasks.push(connectTask);
  }
};
