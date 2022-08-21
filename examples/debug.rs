use buddy_allocator::{BuddyAllocator, BuddyCollection, BuddyLine, OligarchyCollection};
use std::{alloc::Layout, collections::BTreeSet, mem::MaybeUninit};

fn main() {
    let mut allocator = BuddyAllocator::<16, BuddySet, BuddySet>::new(12);
    allocator.init();
    println!();
    allocator.allocate(Layout::new::<usize>()).unwrap_err();
    println!();
    allocator.deallocate(0x1000..0x8000_0000);
    println!();
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

    fn set_order(&mut self, order: usize) {
        println!("Buddies[{order}] set order = {order}");
        self.set = MaybeUninit::new(BTreeSet::new());
        self.order = order;
    }

    fn take(&mut self, idx: usize) -> bool {
        println!("Buddies[{}] take at {idx}", self.order);
        unsafe { self.set.assume_init_mut() }.remove(&idx)
    }
}

impl OligarchyCollection for BuddySet {
    fn take_any(&mut self, align_order: usize, count: usize) -> Option<usize> {
        println!("Buddies[{}] take {count} align = {align_order}", self.order);
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
        println!("Buddies[{}] take one align = {align_order}", self.order);
        let set = unsafe { self.set.assume_init_mut() };
        set.iter().next().copied().map(|i| {
            set.remove(&i);
            i
        })
    }

    fn put(&mut self, idx: usize) -> Option<usize> {
        println!("Buddies[{}] put buddy at = {idx}", self.order);
        let set = unsafe { self.set.assume_init_mut() };
        if set.remove(&(idx & !1)) {
            Some(idx >> 1)
        } else {
            set.insert(idx);
            None
        }
    }
}
