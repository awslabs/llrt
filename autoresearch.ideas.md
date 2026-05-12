# Deferred optimizations

## ~~Zero-copy `Blob.stream()` via native `'js`-bearing stream sources~~ — done

Landed as part of the WPT-conformance branch. The plumbing:

* `llrt_utils::array_buffer::shared_array_buffer_view` — creates a fresh
  `ArrayBuffer<'js>` aliasing the source's bytes (via `JS_NewArrayBuffer`
  with a dup'd `JSValue` in `opaque`) and immediately marks it immutable
  via `JS_SetImmutableArrayBuffer`. Immutability is what makes the
  zero-copy path sound: it blocks both consumer mutation and
  `.transfer()` at the JS layer, so the only QuickJS code path that
  loses our `opaque` (transfer → `js_array_buffer_constructor3` with
  `opaque = NULL`) is unreachable from JS.
* `readable_byte_stream_controller_enqueue_bytes_borrowed` (in
  `llrt_stream_web`) — spec-enqueue path forked to skip the mandatory
  `TransferArrayBuffer(chunk)` call. Pending-BYOB transfers and the
  rest of the spec logic are unchanged.
* `Blob.stream()` and `Blob.slice()` use both: `stream()` enqueues an
  immutable view of `self.data` (no memcpy, consumer can't corrupt the
  blob); `slice()` stores an immutable view as the new blob's
  `self.data` (constant-time slice regardless of payload size).
* `Blob.arrayBuffer()` and `Blob.bytes()` deliberately keep their
  per-call memcpy via `ArrayBuffer::new_copy` — spec mandates a fresh
  *mutable* buffer, and immutable-aliased would observably reject
  consumer writes that the spec requires to succeed (verified against
  Node 24 behaviour).

Measured: `stream()` 32 MiB ×20 = ~7 ms (was ~150 ms); `slice()` is
constant-time. WPT 169/11/180 and unit 994/0/5/999 unchanged.

## Open: zero-copy stream-source for `fetch` body objects

`modules/llrt_fetch/src/body_helpers.rs::create_body_value_stream`
still does `Vec<u8>` materialise + `ArrayBuffer::new_copy` per chunk.
Attempted to apply the same `shared_array_buffer_view` +
`enqueue_bytes_borrowed` treatment as `Blob.stream()`; result *worked*
for the simple consume path but broke `Request` cloning.

Root cause: `tee_readable_stream` is itself a producer-consumer that,
for byte streams, calls the spec `readable_byte_stream_controller_enqueue`
(with `TransferArrayBuffer`) on the chunk it received. When that chunk
aliases an immutable buffer (because the upstream producer used
`enqueue_bytes_borrowed`), the transfer throws `TypeError: ArrayBuffer
is immutable` and the derived `Request`'s body stream dies.

Fixing this properly requires either:

1. **Detect-and-clone in tee** — when about to transfer a chunk whose
   buffer is immutable (or whose backing isn't "owned" in the spec
   sense), copy via `clone_as_uint8_array` + `enqueue` instead of
   `enqueue` + `transfer`. Self-contained change in
   `modules/llrt_stream_web/src/readable/stream/tee.rs` that lets
   immutable chunks flow through tee transparently.
2. **Tee uses `enqueue_borrowed` too** — each branch gets views of the
   shared immutable backing. No copy at all. But the two branches now
   share storage with the source, which complicates `cancel()`
   semantics (cancelling one branch shouldn't free the source for the
   other).
3. **Producer-controlled immutability flag** — the body-stream pull
   algorithm hands out non-immutable buffers (mutable, transferable)
   so tee's spec path works, at the cost of giving up the safety guard
   the immutable flag provides. Would require either accepting the
   transfer-panic risk (mutable shared view, opaque lost on transfer)
   or paying the memcpy.

Probably (1) is the right move — it generalises the `'js`-stream
source concept to consumer-side machinery. Deferred until someone
actually has a profile showing `create_body_value_stream` on the hot
path.

## Other ideas

- (add bullets here as they come up)
