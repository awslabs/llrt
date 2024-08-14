declare module "llrt:uuid" {
  export const NIL: string;

  export const MAX: string;

  export function v1(): string;

  export function v3(name: string, namespace: Array<any> | Uint8Array | String): string;

  export function v4(): string;

  export function v5(name: string, namespace: Array<any> | Uint8Array | String): string;

  export function v6(): string;

  export function v7(): string;

  export function v1ToV6(value: string): Uint8Array;

  export function v6ToV1(value: string): Uint8Array;

  export function parse(value: string): Uint8Array;

  export function stringify(arr: Array<any> | Uint8Array): string;

  export function validate(arr: string): boolean;

  export function version(arr: Array<any> | Uint8Array): number;
}
