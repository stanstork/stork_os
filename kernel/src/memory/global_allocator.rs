use super::heap::heap::Heap;
use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    ptr::NonNull,
};

// GlobalAllocator encapsulates a Heap instance to manage memory allocations.
pub struct GlobalAllocator(UnsafeCell<Heap<32>>);

impl GlobalAllocator {
    pub const fn new() -> Self {
        GlobalAllocator(UnsafeCell::new(Heap::new()))
    }

    // Initializes the GlobalAllocator with a given Heap instance.
    // This function can be used to set up the allocator with a pre-configured Heap.
    pub fn init(&mut self, heap: Heap<32>) {
        self.0 = UnsafeCell::new(heap);
    }

    pub fn alloc_page(&self) -> *mut u8 {
        let heap = unsafe { &mut *self.0.get() };
        let layout = Layout::from_size_align(4096, 4096).unwrap();
        match heap.alloc(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => panic!("Out of memory"),
        }
    }
}

// Implements the GlobalAlloc trait for GlobalAllocator, allowing it to be used as the allocator for the system.
unsafe impl GlobalAlloc for GlobalAllocator {
    // Provides memory allocation using the encapsulated Heap.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Obtain a mutable reference to the Heap.
        let heap = &mut *self.0.get();

        // Allocate memory using the Heap, and handle the result.
        match heap.alloc(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => panic!("Out of memory"), // Panic if the heap cannot fulfill the allocation request.
        }
    }

    // Provides memory deallocation using the encapsulated Heap.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Obtain a mutable reference to the Heap.
        let heap = &mut *self.0.get();

        // Safely convert the raw pointer to NonNull and deallocate the memory.
        if let Some(non_null_ptr) = NonNull::new(ptr) {
            heap.dealloc(non_null_ptr, layout);
        }
    }
}

// The Sync trait implementation is marked unsafe because the GlobalAllocator
// allows mutable static access to the Heap, and it's the developer's responsibility
// to ensure no data races occur.
unsafe impl Sync for GlobalAllocator {}
