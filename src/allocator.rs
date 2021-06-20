use std::alloc::{GlobalAlloc, Layout};

use bumpalo::Bump;

#[derive(Debug, Clone)]
pub struct BumpaloPatriciaAllocator<'a>(pub &'a Bump);

unsafe impl<'a> GlobalAlloc for BumpaloPatriciaAllocator<'a> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.alloc_layout(layout).as_ptr()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // bumpalo cannot deallocate
    }
}
