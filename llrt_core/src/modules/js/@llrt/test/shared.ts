export type SocketReqMsg =
  | ReadyReqMsg
  | NextReqMsg
  | ModuleReqMsg
  | EndReqMsg
  | StartReqMsg
  | CompletedReqMsg
  | ErrorReqMsg;

export type ReadyReqMsg = {
  type: "ready";
  workerId: number;
};

export type ErrorReqMsg = {
  type: "error";
  error: any;
  ended: number;
  started: number;
};

export type ModuleReqMsg = {
  type: "module";
  testCount: number;
  skipCount: number;
  onlyCount: number;
};

export type CompletedReqMsg = {
  type: "completed";
};

export type NextReqMsg = {
  type: "next";
};

export type EndReqMsg = {
  type: "end";
  ended: number;
  started: number;
  isSuite: boolean;
};

export type StartReqMsg = {
  type: "start";
  desc: string;
  isSuite: boolean;
  started: number;
  timeout?: number;
};

export type SocketResponseMap = {
  next: {
    nextFile: string | null;
  };
};

export type SocketRes<T extends SocketReqMsg> = T extends {
  type: keyof SocketResponseMap;
}
  ? SocketResponseMap[T["type"]]
  : null;
