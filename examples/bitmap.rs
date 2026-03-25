use customizable_buddy::{BuddyAllocator, LinkedListBuddy, UsizeBuddy};
use std::ptr::{NonNull, addr_of_mut};

type Allocator<const N: usize> = BuddyAllocator<N, UsizeBuddy, LinkedListBuddy>;

#[repr(C, align(4096))]
struct Page([u8; 4096]);

impl Page {
    const ZERO: Self = Self([0; 4096]);
}

static mut MEMORY: Page = Page::ZERO;

fn main() {
    let mut allocator = Allocator::<7>::new();
    let ptr = NonNull::new(addr_of_mut!(MEMORY).cast::<u8>()).unwrap();
    let len = size_of::<Page>();
    allocator.init(3, ptr);
    unsafe { allocator.transfer(ptr, len) };
    println!("{allocator:?}");
    let (_, size) = allocator.allocate_type::<usize>().unwrap();
    assert_eq!(size, 8);
    println!("{allocator:?}");
}
