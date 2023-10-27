use std::{
    alloc::{GlobalAlloc, Layout},
    mem::size_of,
    ptr::null_mut,
};

use rquickjs::allocator::{Allocator, RawMemPtr};

use crate::GLOBAL;

#[cfg(target_pointer_width = "32")]
const ALLOC_ALIGN: usize = 4;

#[cfg(target_pointer_width = "64")]
const ALLOC_ALIGN: usize = 8;

#[derive(Copy, Clone)]
#[repr(transparent)]
struct Header {
    size: usize,
}

const HEADER_SIZE: usize = size_of::<Header>();
const HEADER_OFFSET: isize = HEADER_SIZE as _;

#[inline]
fn round_size(size: usize) -> usize {
    // this will be optimized by the compiler
    // to something like (size + <off>) & <mask>
    (size + ALLOC_ALIGN - 1) / ALLOC_ALIGN * ALLOC_ALIGN
}

/// The allocator which uses Rust global allocator
pub struct MimallocAllocator;

impl Allocator for MimallocAllocator {
    fn alloc(&mut self, size: usize) -> RawMemPtr {
        let size = round_size(size);
        let alloc_size = size + HEADER_SIZE;
        let layout = if let Ok(layout) = Layout::from_size_align(alloc_size, ALLOC_ALIGN) {
            layout
        } else {
            return null_mut();
        };

        let ptr = unsafe { GLOBAL.alloc(layout) };

        if ptr.is_null() {
            return null_mut();
        }
        {
            let header = unsafe { &mut *(ptr as *mut Header) };
            header.size = size;
        }

        unsafe { ptr.offset(HEADER_OFFSET) }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn dealloc(&mut self, ptr: RawMemPtr) {
        let ptr = unsafe { ptr.offset(-HEADER_OFFSET) };
        let alloc_size = {
            let header = unsafe { &*(ptr as *const Header) };
            header.size + HEADER_SIZE
        };
        let layout = unsafe { Layout::from_size_align_unchecked(alloc_size, ALLOC_ALIGN) };

        unsafe { GLOBAL.dealloc(ptr, layout) };
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn realloc(&mut self, ptr: RawMemPtr, new_size: usize) -> RawMemPtr {
        let new_size = round_size(new_size);
        let ptr = unsafe { ptr.offset(-HEADER_OFFSET) };
        let alloc_size = {
            let header = unsafe { &*(ptr as *const Header) };
            header.size + HEADER_SIZE
        };
        let layout = unsafe { Layout::from_size_align_unchecked(alloc_size, ALLOC_ALIGN) };

        let new_alloc_size = new_size + HEADER_SIZE;

        let ptr = unsafe { GLOBAL.realloc(ptr, layout, new_alloc_size) };

        if ptr.is_null() {
            return null_mut();
        }
        {
            let header = unsafe { &mut *(ptr as *mut Header) };
            header.size = new_size;
        }

        unsafe { ptr.offset(HEADER_OFFSET) }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn usable_size(ptr: RawMemPtr) -> usize {
        let ptr = unsafe { ptr.offset(-HEADER_OFFSET) };
        let header = unsafe { &*(ptr as *const Header) };
        header.size
    }
}
