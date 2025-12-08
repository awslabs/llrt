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

declare module "llrt:timezone" {
  interface Timezone {
    /**
     * Get the UTC offset in minutes for a timezone at a given time.
     *
     * @param timezone - IANA timezone name (e.g., "America/Denver", "Asia/Tokyo")
     * @param epochMs - Unix timestamp in milliseconds
     * @returns UTC offset in minutes (positive = ahead of UTC, negative = behind)
     *
     * @example
     * // Get current offset for Denver (handles DST automatically)
     * const offset = Timezone.getOffset("America/Denver", Date.now());
     * // Returns -420 (UTC-7) in winter, -360 (UTC-6) in summer
     */
    getOffset(timezone: string, epochMs: number): number;

    /**
     * List all available IANA timezone names.
     *
     * @returns Array of timezone names
     *
     * @example
     * const zones = Timezone.list();
     * // ["Africa/Abidjan", "Africa/Accra", ..., "Pacific/Wallis"]
     */
    list(): string[];

    readonly [Symbol.toStringTag]: "Timezone";
  }

  export const Timezone: Timezone;
  export default { Timezone: Timezone };
}
