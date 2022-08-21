use crate::{BuddyCollection, BuddyLine, OligarchyCollection};
use core::fmt;

/// 基于平衡二叉查找树的侵入式伙伴行。
pub struct AvlBuddy {
    base: usize,
    order: usize,
}

impl BuddyLine for AvlBuddy {
    const MIN_ORDER: usize = 0;

    const EMPTY: Self = Self { base: 0, order: 0 };

    #[inline]
    fn init(&mut self, order: usize, base: usize) {
        self.base = base;
        self.order = order;
    }

    fn take(&mut self, _idx: usize) -> bool {
        todo!()
    }
}

impl OligarchyCollection for AvlBuddy {
    fn take_any(&mut self, _align_order: usize, _count: usize) -> Option<usize> {
        todo!()
    }

    fn put(&mut self, _idx: usize) {
        todo!()
    }
}

impl BuddyCollection for AvlBuddy {
    fn take_any(&mut self, _align_order: usize) -> Option<usize> {
        todo!()
    }

    fn put(&mut self, _idx: usize) -> Option<usize> {
        todo!()
    }
}

impl fmt::Debug for AvlBuddy {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
