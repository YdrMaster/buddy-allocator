use crate::{BuddyCollection, BuddyLine, Intrusive, OligarchyCollection};
use core::fmt;

/// 侵入式链表伙伴行。
pub struct LinkedListBuddy {
    intrusive: Intrusive,
}

impl BuddyLine for LinkedListBuddy {
    const MIN_ORDER: usize = 0;

    const EMPTY: Self = Self {
        intrusive: Intrusive::ZERO,
    };

    #[inline]
    fn init(&mut self, order: usize, base: usize) {
        self.intrusive.init(base, order)
    }

    fn take(&mut self, _idx: usize) -> bool {
        unimplemented!()
    }
}

impl OligarchyCollection for LinkedListBuddy {
    fn take_any(&mut self, _align_order: usize, _count: usize) -> Option<usize> {
        todo!()
    }

    fn put(&mut self, _idx: usize) {
        todo!()
    }
}

impl BuddyCollection for LinkedListBuddy {
    fn take_any(&mut self, _align_order: usize) -> Option<usize> {
        todo!()
    }

    fn put(&mut self, _idx: usize) -> Option<usize> {
        todo!()
    }
}

impl fmt::Debug for LinkedListBuddy {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
