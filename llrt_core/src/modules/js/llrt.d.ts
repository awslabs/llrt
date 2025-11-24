// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
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

declare module "qjs" {
  interface MemoryInfo {
    malloc_size: number;
    malloc_limit: number;
    memory_used_size: number;
    malloc_count: number;
    memory_used_count: number;
    atom_count: number;
    atom_size: number;
    str_count: number;
    str_size: number;
    obj_count: number;
    obj_size: number;
    prop_count: number;
    prop_size: number;
    shape_count: number;
    shape_size: number;
    js_func_count: number;
    js_func_size: number;
    js_func_code_size: number;
    js_func_pc2line_count: number;
    js_func_pc2line_size: number;
    c_func_count: number;
    array_count: number;
    fast_array_count: number;
    fast_array_elements: number;
    binary_object_count: number;
    binary_object_size: number;
  }
  function ComputeMemoryUsage(): MemoryInfo;
}
