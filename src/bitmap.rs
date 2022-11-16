use crate::{BuddyCollection, BuddyLine, OligarchyCollection};
use core::fmt;

/// 用一个 usize 作为位图保存占用情况的伙伴行。
///
/// - 非侵入式
/// - 静态分配，容量有限
pub struct UsizeBuddy {
    bits: usize,
    base: usize,
}

impl UsizeBuddy {
    const SIZE: usize = usize::BITS as usize;

    #[inline]
    fn take(&mut self, idx: usize) -> bool {
        let bit = 1usize << idx;
        let bits = self.bits;
        self.bits &= !bit;
        bits & bit == bit
    }
}

impl BuddyLine for UsizeBuddy {
    const EMPTY: Self = Self { bits: 0, base: 0 };

    #[inline]
    fn init(&mut self, _order: usize, base: usize) {
        self.base = base;
    }

    #[inline]
    fn take(&mut self, idx: usize) -> bool {
        self.take(idx - self.base)
    }
}

impl OligarchyCollection for UsizeBuddy {
    #[inline]
    fn take_any(&mut self, align_order: usize, count: usize) -> Option<usize> {
        let count = (1 << count) - 1;
        let align = 1usize << align_order;
        let mut i = 0;
        loop {
            let mask = count << i;
            if self.bits & mask == mask {
                self.bits &= !mask;
                return Some(self.base + i);
            }
            i += align;
            if i >= usize::BITS as usize {
                return None;
            }
        }
    }

    #[inline]
    fn put(&mut self, idx: usize) {
        self.bits |= 1 << (idx - self.base);
    }
}

impl BuddyCollection for UsizeBuddy {
    #[inline]
    fn take_any(&mut self, align_order: usize) -> Option<usize> {
        let align = 1usize << align_order;
        let mut i = 0;
        loop {
            let mask = 1 << i;
            if self.bits & mask == mask {
                self.bits &= !mask;
                return Some(self.base + i);
            }
            i += align;
            if i >= usize::BITS as usize {
                return None;
            }
        }
    }

    #[inline]
    fn put(&mut self, idx: usize) -> Option<usize> {
        let idx = idx - self.base;
        debug_assert!(idx < Self::SIZE, "index out of bound");
        let buddy = idx ^ 1;
        if self.take(buddy) {
            Some(idx << 1)
        } else {
            self.bits |= 1 << idx;
            None
        }
    }
}

impl fmt::Debug for UsizeBuddy {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:b}", self.bits)
    }
}
