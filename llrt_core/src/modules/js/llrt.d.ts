// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
declare var __bootstrap: any;

declare namespace NodeJS {
  import assert from "assert";
  interface Global {
    assert: typeof assert;
  }
}

interface Headers {
  entries(): any;
}

declare var assert: NodeJS.Global["assert"];
declare var _require: NodeJS.Global["require"];
declare var __lambdaSetRequestId: (id?: string) => void;

declare var __handler: (data: any) => Promise<any>;
