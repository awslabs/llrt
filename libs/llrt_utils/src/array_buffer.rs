// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//! Zero-copy `ArrayBuffer` helpers built on QuickJS-NG primitives.
//!
//! `rquickjs` doesn't yet ship safe wrappers for QuickJS-NG's
//! [immutable ArrayBuffer](https://tc39.es/proposal-immutable-arraybuffer/)
//! support, so we go through `rquickjs::qjs::*` directly. The two
//! capabilities exposed here are:
//!
//! * [`shared_array_buffer_view`] — create a fresh `ArrayBuffer` that
//!   borrows the bytes of an existing one (no memcpy), kept alive via a
//!   dup'd `JSValue` reference. The view is marked immutable, which is
//!   both a correctness guarantee (consumer mutations can't leak into the
//!   source) and a hard safety rail (the only QuickJS code path that
//!   would lose our refcount handle is `.transfer()`, which immutability
//!   blocks at the JS layer).
//! * [`set_immutable`] — flip the immutable flag on an existing
//!   `ArrayBuffer` (used when the source is a freshly-allocated buffer
//!   we own and want to seal before handing out).
//!
//! These are used by `Blob.stream()` / `Blob.slice()` and by fetch's
//! `Response.body` / `Request.body` getters to hand out aliased,
//! transfer-safe views into producer-owned storage.

use std::ffi::c_void;

use rquickjs::{qjs, ArrayBuffer, Ctx, Exception, Result, Value};

/// Mark an `ArrayBuffer` as immutable: subsequent writes through any
/// `Uint8Array` / `DataView` view silently fail (or `TypeError` in strict
/// mode), and `.transfer()` throws `TypeError: ArrayBuffer is immutable`.
///
/// Calling this on an already-immutable buffer is a no-op. Calling it on
/// a detached buffer is a no-op (QuickJS returns -1 internally). The flag
/// is checked at write/transfer time, not at create time, so the buffer
/// can be initialised with bytes before being sealed.
pub fn set_immutable(ab: &ArrayBuffer<'_>) {
    // Safety: the JSValue is owned by `ab`; we only flip a boolean flag
    // on the underlying `JSArrayBuffer` struct.
    unsafe {
        qjs::JS_SetImmutableArrayBuffer(ab.as_value().as_raw(), true);
    }
}

/// Create a fresh, **immutable** `ArrayBuffer` that shares storage with
/// `source` at `[offset..offset+len]` without copying any bytes. The
/// returned buffer holds a dup'd reference to the source's `JSValue`, so
/// the backing allocation stays alive exactly as long as any view (or
/// transferred descendant of it) is reachable.
///
/// Immutability is what makes this sound:
///
///   * Writes through `Uint8Array` / `DataView` views silently no-op
///     (strict mode: `TypeError`) — aliased consumers can't corrupt the
///     source.
///   * `buffer.transfer()` throws `TypeError: ArrayBuffer is immutable`
///     — so a consumer can't detach the view and drop the `opaque`
///     pointer that keeps the source alive. Without this guard the
///     `free_func` would later fire with `opaque=NULL` (QuickJS strips
///     `opaque` on transfer; see `js_array_buffer_constructor3`) and
///     panic in `Box::from_raw(null)`. Because immutability blocks
///     transfer at the JS layer, that path is unreachable.
///
/// If a future caller wants a *mutable* shared view, they need a
/// different cleanup strategy (ptr-keyed side table, upstream QuickJS
/// patch, or accepting a per-transfer leak).
pub fn shared_array_buffer_view<'js>(
    ctx: &Ctx<'js>,
    source: &ArrayBuffer<'js>,
    offset: usize,
    len: usize,
) -> Result<ArrayBuffer<'js>> {
    let raw = source
        .as_raw()
        .ok_or_else(|| Exception::throw_type(ctx, "cannot view a detached ArrayBuffer"))?;
    debug_assert!(
        offset.checked_add(len).is_some_and(|e| e <= raw.len),
        "shared_array_buffer_view: slice out of range"
    );
    let ptr = unsafe { raw.ptr.as_ptr().add(offset) };

    // Dup the source's JSValue. The returned ArrayBuffer's free-callback
    // (below) will drop this reference.
    let ctx_ptr = ctx.as_raw().as_ptr();
    let rt = unsafe { qjs::JS_GetRuntime(ctx_ptr) };
    let source_val = unsafe { qjs::JS_DupValueRT(rt, source.as_value().as_raw()) };
    let opaque = Box::into_raw(Box::new(source_val)) as *mut c_void;

    extern "C" fn free_shared(rt: *mut qjs::JSRuntime, opaque: *mut c_void, _ptr: *mut c_void) {
        // `opaque` is guaranteed non-null: the only QuickJS code path
        // that loses it is `.transfer()`, which is blocked by the
        // immutability flag we set below.
        unsafe {
            let boxed = Box::from_raw(opaque as *mut qjs::JSValue);
            qjs::JS_FreeValueRT(rt, *boxed);
        }
    }

    let view = unsafe {
        let val = qjs::JS_NewArrayBuffer(
            ctx_ptr,
            ptr,
            len as _,
            Some(free_shared),
            opaque,
            /*is_shared=*/ false,
        );
        if qjs::JS_IsException(val) {
            // QuickJS didn't take ownership of `opaque`; drop it ourselves.
            let boxed = Box::from_raw(opaque as *mut qjs::JSValue);
            qjs::JS_FreeValueRT(rt, *boxed);
            return Err(ctx.throw(ctx.catch()));
        }
        let value = Value::from_raw(ctx.clone(), val);
        ArrayBuffer::from_value(value)
            .ok_or_else(|| Exception::throw_type(ctx, "expected ArrayBuffer"))?
    };

    set_immutable(&view);
    Ok(view)
}
