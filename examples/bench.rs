use customizable_buddy::{BuddyAllocator, BuddyError, AvlBuddy, UsizeBuddy};
use std::{
    alloc::Layout,
    ptr::{null_mut, NonNull},
    time::Instant,
};

type Allocator<const N: usize> = BuddyAllocator<N, UsizeBuddy, AvlBuddy>;

#[repr(C, align(4096))]
struct Page([u8; 4096]);

impl Page {
    const ZERO: Self = Self([0; 4096]);
}

/// 256 MiB
static mut MEMORY: [Page; 65536] = [Page::ZERO; 65536];

fn main() -> Result<(), BuddyError> {
    let mut allocator = Allocator::<12>::new();
    let ptr = NonNull::new(unsafe { MEMORY.as_mut_ptr() }).unwrap();
    let len = core::mem::size_of_val(unsafe { &MEMORY });
    allocator.init(12, ptr);
    println!(
        "MEMORY: {:#x}..{:#x}",
        ptr.as_ptr() as usize,
        ptr.as_ptr() as usize + len
    );
    let t = Instant::now();
    unsafe { allocator.transfer(ptr, len) };
    println!("transfer {:?}", t.elapsed());

    assert_eq!(len, allocator.capacity());
    assert_eq!(len, allocator.free());

    println!(
        "
BEFORE
{allocator:#x?}"
    );

    let mut blocks = [null_mut::<Page>(); 65536];
    let layout = Layout::new::<Page>();
    let t = Instant::now();
    for block in blocks.iter_mut() {
        let (ptr, size) = allocator.allocate_type::<Page>()?;
        debug_assert_eq!(layout.size(), size);
        *block = ptr.as_ptr();
    }
    let ta = t.elapsed();

    println!(
        "
EMPTY
{allocator:#x?}"
    );

    assert_eq!(len, allocator.capacity());
    assert_eq!(len - blocks.len() * layout.size(), allocator.free());

    let t = Instant::now();
    for block in blocks.iter_mut() {
        allocator.deallocate(NonNull::new(*block).unwrap(), layout.size());
        *block = null_mut();
    }
    let td = t.elapsed();

    assert_eq!(len, allocator.capacity());
    assert_eq!(len, allocator.free());

    println!(
        "
AFTER
{allocator:#x?}"
    );

    println!(
        "allocate   {:?} ({} times)",
        ta / blocks.len() as u32,
        blocks.len()
    );
    println!(
        "deallocate {:?} ({} times)",
        td / blocks.len() as u32,
        blocks.len()
    );

    Ok(())
}
