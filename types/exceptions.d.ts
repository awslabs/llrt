export {};

declare global {
  interface Error {
    name: string;
    message: string;
    stack?: string;
  }

  interface ErrorConstructor {
    new (message?: string): Error;
    (message?: string): Error;
    readonly prototype: Error;
  }

  interface DOMException extends Error {
    readonly message: string;
    readonly name: string;
    readonly stack: string;
  }
  var Error: ErrorConstructor;
  var DOMException: {
    prototype: DOMException;
    new (message?: string, name?: string): DOMException;
  };
}
