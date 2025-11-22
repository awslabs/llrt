/**
 * The `dns` module enables name resolution. For example, use it to look up IP
 * addresses of host names.
 *
 * Although named for the [Domain Name System (DNS)](https://en.wikipedia.org/wiki/Domain_Name_System), it does not always use the
 * DNS protocol for lookups. {@link lookup} uses the operating system
 * facilities to perform name resolution. It may not need to perform any network
 * communication. To perform name resolution the way other applications on the same
 * system do, use {@link lookup}.
 *
 * ```js
 * import dns from 'dns';
 *
 * dns.lookup('example.org', (err, address, family) => {
 *   console.log('address: %j family: IPv%s', address, family);
 * });
 * // address: "93.184.216.34" family: IPv4
 * ```
 *
 */
declare module "dns" {
  export interface LookupOptions {
    /**
     * The record family. Must be `4`, `6`, or `0`.
     * The value 0 indicates that either an IPv4 or IPv6 address is returned.
     * @default 0
     */
    family?: number | "IPv4" | "IPv6" | undefined;
    /**
     * When `true`, the callback returns all resolved addresses in an array. Otherwise, returns a single address.
     * @default false
     */
    all?: boolean | undefined;
    /**
     * When `verbatim`, the resolved addresses are return unsorted. When `ipv4first`, the resolved addresses are sorted
     * by placing IPv4 addresses before IPv6 addresses. When `ipv6first`, the resolved addresses are sorted by placing IPv6
     * addresses before IPv4 addresses. Default value is configurable using
     * {@link setDefaultResultOrder} or [`--dns-result-order`](https://nodejs.org/docs/latest-v20.x/api/cli.html#--dns-result-orderorder).
     * @default `verbatim` (addresses are not reordered)
     */
    order?: "verbatim" | "ipv4first" | "ipv6first" | undefined;
  }
  export interface LookupOneOptions extends LookupOptions {
    all?: false | undefined;
  }
  export interface LookupAllOptions extends LookupOptions {
    all: true;
  }
  export interface LookupAddress {
    /**
     * A string representation of an IPv4 or IPv6 address.
     */
    address: string;
    /**
     * `4` or `6`, denoting the family of `address`, or `0` if the address is not an IPv4 or IPv6 address. `0` is a likely indicator of a
     * bug in the name resolution service used by the operating system.
     */
    family: number;
  }
  /**
   * Resolves a host name (e.g. `'nodejs.org'`) into the first found A (IPv4) or
   * AAAA (IPv6) record. All `option` properties are optional. If `options` is an
   * integer, then it must be `4` or `6` – if `options` is `0` or not provided, then
   * IPv4 and IPv6 addresses are both returned if found.
   *
   * On error, `err` is an `Error` object, where `err.code` is the error code.
   * Keep in mind that `err.code` will be set to `'ENOTFOUND'` not only when
   * the host name does not exist but also when the lookup fails in other ways
   * such as no available file descriptors.
   *
   * `dns.lookup()` does not necessarily have anything to do with the DNS protocol.
   * The implementation uses an operating system facility that can associate names
   * with addresses and vice versa.
   *
   * Example usage:
   *
   * ```js
   * import dns from 'dns';
   * const options = {
   *   family: 6,
   * };
   * dns.lookup('example.com', options, (err, address, family) =>
   *   console.log('address: %j family: IPv%s', address, family));
   * // address: "2606:2800:220:1:248:1893:25c8:1946" family: IPv6
   *
   * ```
   *
   */
  export function lookup(
    hostname: string,
    family: number,
    callback: (
      err: DOMException | null,
      address: string,
      family: number
    ) => void
  ): void;
  export function lookup(
    hostname: string,
    options: LookupOneOptions,
    callback: (err: Error | null, address: string, family: number) => void
  ): void;
  export function lookup(
    hostname: string,
    options: LookupAllOptions,
    callback: (err: Error | null, addresses: LookupAddress[]) => void
  ): void;
  export function lookup(
    hostname: string,
    options: LookupOptions,
    callback: (
      err: Error | null,
      address: string | LookupAddress[],
      family: number
    ) => void
  ): void;
  export function lookup(
    hostname: string,
    callback: (err: Error | null, address: string, family: number) => void
  ): void;
}
declare module "dns" {
  export * from "dns";
}
