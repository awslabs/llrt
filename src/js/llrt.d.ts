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

  export class XmlText {
    constructor(private value: string) {}
    toString(): string;
  }

  export class XmlNode {
    readonly children: any[];
    static of(name: string, childText?: string, withName?: string): XmlNode;
    constructor(name: string, children?: any[]);
    withName(name: string): XmlNode;
    addAttribute(name: string, value: any): XmlNode;
    addChildNode(child: any): XmlNode;
    removeAttribute(name: string): XmlNode;
    toString(): string;
  }
}
