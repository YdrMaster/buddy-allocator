use buddy_allocator::{BuddyAllocator, LinkedListBuddy, UsizeBuddy};
use std::ptr::NonNull;

type Allocator<const N: usize> = BuddyAllocator<N, UsizeBuddy, LinkedListBuddy>;

#[repr(C, align(4096))]
struct Page([u8; 4096]);

impl Page {
    const ZERO: Self = Self([0; 4096]);
}

static mut MEMORY: Page = Page::ZERO;

fn main() {
    let mut allocator = Allocator::<7>::new();
    let ptr = NonNull::new(unsafe { MEMORY.0.as_mut_ptr() }).unwrap();
    let len = core::mem::size_of_val(unsafe { &MEMORY });
    allocator.init(3, ptr);
    unsafe { allocator.transfer(ptr, len) };
    println!("{allocator:?}");
    let (_0, size) = allocator.allocate_type::<usize>().unwrap();
    assert_eq!(size, 8);
    println!("{allocator:?}");
}
