use buddy_allocator::{BitArrayBuddy, BuddyAllocator, LinkedListBuddy};
use std::ptr::NonNull;

type Allocator<const N: usize> = BuddyAllocator<N, BitArrayBuddy, LinkedListBuddy>;

#[repr(C, align(4096))]
struct Page([u8; 4096]);

impl Page {
    const ZERO: Self = Self([0; 4096]);
}

/// 8 MiB
static mut MEMORY: [Page; 2048] = [Page::ZERO; 2048];

fn main() {
    let mut allocator = Allocator::<5>::new();
    let ptr = NonNull::new(unsafe { MEMORY.as_mut_ptr() }).unwrap();
    let len = core::mem::size_of_val(unsafe { &MEMORY });
    allocator.init(12, ptr);
    println!(
        "MEMORY: {:#x}..{:#x}",
        ptr.as_ptr() as usize,
        ptr.as_ptr() as usize + len
    );
    for i in ptr.as_ptr() as usize..ptr.as_ptr() as usize + len {
        unsafe { *(i as *mut u8) = 0xff };
    }
    allocator.deallocate(ptr, len);
    println!("Hello, world!");
}
