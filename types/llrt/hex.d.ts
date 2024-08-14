declare module "llrt:hex" {
  export const decode: (text: string) => Uint8Array;
  export const encode: (bytes: Uint8Array) => string;
}
