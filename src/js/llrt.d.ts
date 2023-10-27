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

declare var __handler: (data: any) => Promise<any>;

declare module "hex" {
  export const decode: (text: string) => Uint8Array;
  export const encode: (bytes: Uint8Array) => string;
}

declare module "xml" {
  export class XMLParser {
    constructor(options?: {
      ignoreAttributes?: boolean;
      attributeNamePrefix?: string;
      textNodeName?: string;
      attributeValueProcessor?: (
        attrName: string,
        attrValue: string,
        jpath: string
      ) => unknown;
      tagValueProcessor?: (
        attrName: string,
        attrValue: string,
        jpath: string,
        hasAttributes: boolean
      ) => unknown;
    });
    parse(xml: string): any;
  }
}
