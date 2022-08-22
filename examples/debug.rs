use buddy_allocator::{BuddyAllocator, BuddyCollection, BuddyLine, OligarchyCollection};
use std::{alloc::Layout, collections::BTreeSet, fmt, mem::MaybeUninit, ptr::NonNull};

fn main() {
    let mut allocator = BuddyAllocator::<16, BuddySet, BuddySet>::new();
    allocator.init(12, non_null(0x1000));
    println!();
    assert!(allocator.allocate(Layout::new::<usize>()).is_err());
    println!();
    allocator.deallocate(non_null(0x1000), 0x7fff_f000);

    println!();
    println!("{allocator:?}");
    let (ptr0, size0) = allocator.allocate(Layout::new::<[u8; 2048]>()).unwrap();
    println!("{allocator:?}");
    let (ptr1, size1) = allocator.allocate(Layout::new::<[u8; 4096]>()).unwrap();
    println!("{allocator:?}");
    let (ptr2, size2) = allocator
        .allocate(Layout::new::<[u8; 4096 * 3 - 1]>())
        .unwrap();
    println!("{allocator:?}");

    assert_eq!(4096, size0);
    assert_eq!(4096, size1);
    assert_eq!(4096 * 3, size2);

    println!();
    println!("{allocator:?}");
    allocator.deallocate(ptr0, size0);
    println!("{allocator:?}");
    allocator.deallocate(ptr1, size1);
    println!("{allocator:?}");
    allocator.deallocate(ptr2, size2);
    println!("{allocator:?}");
}

#[inline]
fn non_null(addr: usize) -> NonNull<u8> {
    NonNull::new(addr as *mut _).unwrap()
}

struct BuddySet {
    set: MaybeUninit<BTreeSet<usize>>,
    order: usize,
}

impl BuddyLine for BuddySet {
    const MIN_ORDER: usize = 0;

    const EMPTY: Self = Self {
        set: MaybeUninit::uninit(),
        order: Self::MIN_ORDER,
    };

    fn init(&mut self, order: usize, base: usize) {
        self.order = order;
        println!("Buddies[{order}] init as base = {base} order = {order}");
    }

    fn take(&mut self, idx: usize) -> bool {
        println!("Buddies[{}] take at {idx}", self.order);
        unsafe { self.set.assume_init_mut() }.remove(&idx)
    }
}

impl OligarchyCollection for BuddySet {
    fn take_any(&mut self, align_order: usize, count: usize) -> Option<usize> {
        println!(
            "Buddies[{}] take {count} with align = {align_order}",
            self.order
        );
        assert!(count == 1);
        let set = unsafe { self.set.assume_init_mut() };
        set.iter().next().copied().map(|i| {
            set.remove(&i);
            i
        })
    }

    fn put(&mut self, idx: usize) {
        println!("Buddies[{}] put oligarchy at {idx}", self.order);

        unsafe { self.set.assume_init_mut() }.insert(idx);
    }
}

impl BuddyCollection for BuddySet {
    fn take_any(&mut self, align_order: usize) -> Option<usize> {
        println!("Buddies[{}] take 1 with align = {align_order}", self.order);
        let set = unsafe { self.set.assume_init_mut() };
        set.iter().next().copied().map(|i| {
            set.remove(&i);
            i
        })
    }

    fn put(&mut self, idx: usize) -> Option<usize> {
        println!("Buddies[{}] put buddy at = {idx}", self.order);
        let set = unsafe { self.set.assume_init_mut() };
        if set.remove(&(idx ^ 1)) {
            Some(idx >> 1)
        } else {
            set.insert(idx);
            None
        }
    }
}

impl fmt::Debug for BuddySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", unsafe { self.set.assume_init_ref() })
    }
}
