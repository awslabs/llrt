/*
MIT License

Copyright (c) 2021-Present Anthony Fu <https://github.com/antfu>
Copyright (c) 2021-Present Matias Capeletto <https://github.com/patak-dev>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
 */

// Extracted and modified from Vitest:  https://github.com/vitest-dev/vitest/blob/0f86ff98975a80191d6215b0d30ff6cd9f8388d3/packages/utils/src/stringify.ts

import type { PrettyFormatOptions } from "pretty-format";
import {
  format as prettyFormat,
  plugins as prettyFormatPlugins,
} from "pretty-format";

const {
  AsymmetricMatcher,
  DOMCollection,
  DOMElement,
  Immutable,
  ReactElement,
  ReactTestComponent,
} = prettyFormatPlugins;

const PLUGINS = [
  ReactTestComponent,
  ReactElement,
  DOMElement,
  DOMCollection,
  Immutable,
  AsymmetricMatcher,
];

export function stringify(
  object: unknown,
  maxDepth = 10,
  { maxLength, ...options }: PrettyFormatOptions & { maxLength?: number } = {}
): string {
  const MAX_LENGTH = maxLength ?? 10000;
  let result;

  try {
    result = prettyFormat(object, {
      maxDepth,
      escapeString: false,
      // min: true,
      plugins: PLUGINS,
      ...options,
    });
  } catch {
    result = prettyFormat(object, {
      callToJSON: false,
      maxDepth,
      escapeString: false,
      // min: true,
      plugins: PLUGINS,
      ...options,
    });
  }

  return result.length >= MAX_LENGTH && maxDepth > 1
    ? stringify(object, Math.floor(maxDepth / 2))
    : result;
}
