use std::rc::Rc;
use std::alloc::{GlobalAlloc, Layout};

use bumpalo::Bump;

#[derive(Debug, Clone, Copy)]
pub struct BumpaloPatriciaAllocator(Rc<Bump>);

unsafe impl GlobalAlloc for BumpaloPatriciaAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.alloc_layout(layout).as_ptr()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // bumpalo cannot deallocate
    }
}
