use libmimalloc_sys as ffi;

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;
use core::sync::atomic::AtomicUsize;
use ffi::*;
use std::sync::atomic::Ordering;

pub static USED_MEM: AtomicUsize = AtomicUsize::new(0);

pub struct TrackingMiMalloc;

unsafe impl GlobalAlloc for TrackingMiMalloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        USED_MEM.fetch_add(size, Ordering::Relaxed);
        mi_malloc_aligned(size, layout.align()) as *mut u8
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        USED_MEM.fetch_add(size, Ordering::Relaxed);
        mi_zalloc_aligned(size, layout.align()) as *mut u8
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        USED_MEM.fetch_sub(layout.size(), Ordering::Relaxed);
        mi_free(ptr as *mut c_void);
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let size = layout.size();
        if new_size > size {
            USED_MEM.fetch_add(new_size - size, Ordering::Relaxed);
        } else {
            USED_MEM.fetch_sub(size - new_size, Ordering::Relaxed);
        }
        mi_realloc_aligned(ptr as *mut c_void, new_size, layout.align()) as *mut u8
    }
}
