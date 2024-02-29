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

// Extracted and modified from Vitest:  https://github.com/vitest-dev/vitest/blob/a199ac2dd1322d7839d4d1350c983070da546805/packages/expect/src/jest-expect.ts

import {
    arrayBufferEquality, equals as jestEquals, generateToBeMessage,
    iterableEquality,
    sparseArrayEquality, subsetEquality,
    typeEquality
} from "./jest-utils";
import ChaiPlugin = Chai.ChaiPlugin;
import Assertion = Chai.Assertion;

// Jest Expect Compact
export const JestChaiExpect: ChaiPlugin = (chai, utils) => {
    const {AssertionError} = chai
    // const c = () => getColors()
    const customTesters: ((a: any, b: any, customTesters?: Array<any>, aStack?: Array<any>, bStack?: Array<any>) => (boolean | undefined))[] = []

    function def(name: string | (string)[], fn: ((this: Chai.AssertionStatic & Assertion, ...args: any[]) => any)) {
        const addMethod = (n: string) => {
            // const softWrapper = wrapSoft(utils, fn)
            utils.addMethod(chai.Assertion.prototype, n, fn)
            // utils.addMethod((globalThis as any)[JEST_MATCHERS_OBJECT].matchers, n, softWrapper)
        }

        if (Array.isArray(name))
            name.forEach(n => addMethod(n))

        else
            addMethod(name)
    }

    (['throw', 'throws', 'Throw'] as const).forEach((m) => {
        utils.overwriteMethod(chai.Assertion.prototype, m, (_super: any) => {
            return function (this: Chai.Assertion & Chai.AssertionStatic, ...args: any[]) {
                const promise = utils.flag(this, 'promise')
                const object = utils.flag(this, 'object')
                const isNot = utils.flag(this, 'negate') as boolean
                if (promise === 'rejects') {
                    utils.flag(this, 'object', () => {
                        throw object
                    })
                }
                    // if it got here, it's already resolved
                    // unless it tries to resolve to a function that should throw
                // called as '.resolves[.not].toThrow()`
                else if (promise === 'resolves' && typeof object !== 'function') {
                    if (!isNot) {
                        const message = utils.flag(this, 'message') || 'expected promise to throw an error, but it didn\'t'
                        const error = {
                            showDiff: false,
                        }
                        throw new AssertionError(message, error, utils.flag(this, 'ssfi'))
                    } else {
                        return
                    }
                }
                _super.apply(this, args)
            }
        })
    })

    def('toEqual', function (expected) {
        const actual = utils.flag(this, 'object')
        const equal = jestEquals(
            actual,
            expected,
            [...customTesters, iterableEquality],
        )
        return this.assert(
            equal,
            'expected #{this} to deeply equal #{exp}',
            'expected #{this} to not deeply equal #{exp}',
            expected,
            actual,
        )
    })

    def('toStrictEqual', function (expected) {
        const obj = utils.flag(this, 'object')
        const equal = jestEquals(
            obj,
            expected,
            [
                ...customTesters,
                iterableEquality,
                typeEquality,
                sparseArrayEquality,
                arrayBufferEquality,
            ],
            true,
        )

        return this.assert(
            equal,
            'expected #{this} to strictly equal #{exp}',
            'expected #{this} to not strictly equal #{exp}',
            expected,
            obj,
        )
    })
    def('toBe', function (expected) {
        const actual = this._obj
        const pass = Object.is(actual, expected)

        let deepEqualityName = ''

        if (!pass) {
            const toStrictEqualPass = jestEquals(
                actual,
                expected,
                [
                    ...customTesters,
                    iterableEquality,
                    typeEquality,
                    sparseArrayEquality,
                    arrayBufferEquality,
                ],
                true,
            )

            if (toStrictEqualPass) {
                deepEqualityName = 'toStrictEqual'
            } else {
                const toEqualPass = jestEquals(
                    actual,
                    expected,
                    [...customTesters, iterableEquality],
                )

                if (toEqualPass)
                    deepEqualityName = 'toEqual'
            }
        }

        return this.assert(
            pass,
            generateToBeMessage(deepEqualityName),
            'expected #{this} not to be #{exp} // Object.is equality',
            expected,
            actual,
        )
    })
    def('toMatchObject', function (expected) {
        const actual = this._obj
        return this.assert(
            jestEquals(actual, expected, [...customTesters, iterableEquality, subsetEquality]),
            'expected #{this} to match object #{exp}',
            'expected #{this} to not match object #{exp}',
            expected,
            actual,
        )
    })
    def('toMatch', function (expected: string | RegExp) {
        if (typeof expected === 'string')
            return this.include(expected)
        else
            return this.match(expected)
    })
    def('toContain', function (item) {
        const actual = this._obj as Iterable<unknown> | string

        // make "actual" indexable to have compatibility with jest
        if (actual != null && typeof actual !== 'string')
            utils.flag(this, 'object', Array.from(actual as Iterable<unknown>))
        return this.contain(item)
    })
    def('toContainEqual', function (expected) {
        const obj = utils.flag(this, 'object')
        const index = Array.from(obj).findIndex((item) => {
            return jestEquals(item, expected, customTesters)
        })

        this.assert(
            index !== -1,
            'expected #{this} to deep equally contain #{exp}',
            'expected #{this} to not deep equally contain #{exp}',
            expected,
        )
    })
    def('toBeTruthy', function () {
        const obj = utils.flag(this, 'object')
        this.assert(
            Boolean(obj),
            'expected #{this} to be truthy',
            'expected #{this} to not be truthy',
            obj,
            false,
        )
    })
    def('toBeFalsy', function () {
        const obj = utils.flag(this, 'object')
        this.assert(
            !obj,
            'expected #{this} to be falsy',
            'expected #{this} to not be falsy',
            obj,
            false,
        )
    })
    def('toBeGreaterThan', function (expected: number | bigint) {
        const actual = this._obj as number | bigint
        assertTypes(actual, 'actual', ['number', 'bigint'])
        assertTypes(expected, 'expected', ['number', 'bigint'])
        return this.assert(
            actual > expected,
            `expected ${actual} to be greater than ${expected}`,
            `expected ${actual} to be not greater than ${expected}`,
            actual,
            expected,
            false,
        )
    })
    def('toBeGreaterThanOrEqual', function (expected: number | bigint) {
        const actual = this._obj as number | bigint
        assertTypes(actual, 'actual', ['number', 'bigint'])
        assertTypes(expected, 'expected', ['number', 'bigint'])
        return this.assert(
            actual >= expected,
            `expected ${actual} to be greater than or equal to ${expected}`,
            `expected ${actual} to be not greater than or equal to ${expected}`,
            actual,
            expected,
            false,
        )
    })
    def('toBeLessThan', function (expected: number | bigint) {
        const actual = this._obj as number | bigint
        assertTypes(actual, 'actual', ['number', 'bigint'])
        assertTypes(expected, 'expected', ['number', 'bigint'])
        return this.assert(
            actual < expected,
            `expected ${actual} to be less than ${expected}`,
            `expected ${actual} to be not less than ${expected}`,
            actual,
            expected,
            false,
        )
    })
    def('toBeLessThanOrEqual', function (expected: number | bigint) {
        const actual = this._obj as number | bigint
        assertTypes(actual, 'actual', ['number', 'bigint'])
        assertTypes(expected, 'expected', ['number', 'bigint'])
        return this.assert(
            actual <= expected,
            `expected ${actual} to be less than or equal to ${expected}`,
            `expected ${actual} to be not less than or equal to ${expected}`,
            actual,
            expected,
            false,
        )
    })
    def('toBeNaN', function () {
        return this.be.NaN
    })
    def('toBeUndefined', function () {
        return this.be.undefined
    })
    def('toBeNull', function () {
        return this.be.null
    })
    def('toBeDefined', function () {
        const negate = utils.flag(this, 'negate')
        utils.flag(this, 'negate', false)

        if (negate)
            return this.be.undefined

        return this.not.be.undefined
    })
    def('toBeTypeOf', function (expected: 'bigint' | 'boolean' | 'function' | 'number' | 'object' | 'string' | 'symbol' | 'undefined') {
        const actual = typeof this._obj
        const equal = expected === actual
        return this.assert(
            equal,
            'expected #{this} to be type of #{exp}',
            'expected #{this} not to be type of #{exp}',
            expected,
            actual,
        )
    })
    def('toBeInstanceOf', function (obj: any) {
        return this.instanceOf(obj)
    })
    def('toHaveLength', function (length: number) {
        return this.have.length(length)
    })
    // destructuring, because it checks `arguments` inside, and value is passing as `undefined`
    def('toHaveProperty', function (...args: [property: string | (string | number)[], value?: any]) {
        if (Array.isArray(args[0]))
            args[0] = args[0].map(key => String(key).replace(/([.[\]])/g, '\\$1')).join('.')

        const actual = this._obj as any
        const [propertyName, expected] = args
        const getValue = () => {
            const hasOwn = Object.prototype.hasOwnProperty.call(actual, propertyName)
            if (hasOwn)
                return {value: actual[propertyName], exists: true}
            return utils.getPathInfo(actual, propertyName)
        }
        const {value, exists} = getValue()
        const pass = exists && (args.length === 1 || jestEquals(expected, value, customTesters))

        const valueString = args.length === 1 ? '' : ` with value ${utils.objDisplay(expected)}`

        return this.assert(
            pass,
            `expected #{this} to have property "${propertyName}"${valueString}`,
            `expected #{this} to not have property "${propertyName}"${valueString}`,
            expected,
            exists ? value : undefined,
        )
    })
    def('toBeCloseTo', function (received: number, precision = 2) {
        const expected = this._obj
        let pass = false
        let expectedDiff = 0
        let receivedDiff = 0

        if (received === Number.POSITIVE_INFINITY && expected === Number.POSITIVE_INFINITY) {
            pass = true
        } else if (received === Number.NEGATIVE_INFINITY && expected === Number.NEGATIVE_INFINITY) {
            pass = true
        } else {
            expectedDiff = 10 ** -precision / 2
            receivedDiff = Math.abs(expected - received)
            pass = receivedDiff < expectedDiff
        }
        return this.assert(
            pass,
            `expected #{this} to be close to #{exp}, received difference is ${receivedDiff}, but expected ${expectedDiff}`,
            `expected #{this} to not be close to #{exp}, received difference is ${receivedDiff}, but expected ${expectedDiff}`,
            received,
            expected,
            false,
        )
    })

    const ordinalOf = (i: number) => {
        const j = i % 10
        const k = i % 100

        if (j === 1 && k !== 11)
            return `${i}st`

        if (j === 2 && k !== 12)
            return `${i}nd`

        if (j === 3 && k !== 13)
            return `${i}rd`

        return `${i}th`
    }

    def(['toThrow', 'toThrowError'], function (expected?: string | RegExp | Error) {
        if (typeof expected === 'string' || typeof expected === 'undefined' || expected instanceof RegExp)
            return this.throws(expected)

        const obj = this._obj
        const promise = utils.flag(this, 'promise')
        const isNot = utils.flag(this, 'negate') as boolean
        let thrown: any = null

        if (promise === 'rejects') {
            thrown = obj
        }
            // if it got here, it's already resolved
            // unless it tries to resolve to a function that should throw
        // called as .resolves.toThrow(Error)
        else if (promise === 'resolves' && typeof obj !== 'function') {
            if (!isNot) {
                const message = utils.flag(this, 'message') || 'expected promise to throw an error, but it didn\'t'
                const error = {
                    showDiff: false,
                }
                throw new AssertionError(message, error, utils.flag(this, 'ssfi'))
            } else {
                return
            }
        } else {
            let isThrow = false
            try {
                obj()
            } catch (err) {
                isThrow = true
                thrown = err
            }

            if (!isThrow && !isNot) {
                const message = utils.flag(this, 'message') || 'expected function to throw an error, but it didn\'t'
                const error = {
                    showDiff: false,
                }
                throw new AssertionError(message, error, utils.flag(this, 'ssfi'))
            }
        }

        if (typeof expected === 'function') {
            // @ts-ignore
            const name = expected.name || expected.prototype.constructor.name
            return this.assert(
                thrown && thrown instanceof expected,
                `expected error to be instance of ${name}`,
                `expected error not to be instance of ${name}`,
                expected,
                thrown,
            )
        }

        if (expected instanceof Error) {
            return this.assert(
                thrown && expected.message === thrown.message,
                `expected error to have message: ${expected.message}`,
                `expected error not to have message: ${expected.message}`,
                expected.message,
                thrown && thrown.message,
            )
        }

        throw new Error(`"toThrow" expects string, RegExp, function, Error instance or asymmetric matcher, got "${typeof expected}"`)
    })

    def('toSatisfy', function (matcher: Function, message?: string) {
        return this.be.satisfy(matcher, message)
    })
}


export function assertTypes(value: unknown, name: string, types: string[]): void {
    const receivedType = typeof value
    const pass = types.includes(receivedType)
    if (!pass)
        throw new TypeError(`${name} value must be ${types.join(' or ')}, received "${receivedType}"`)
}
