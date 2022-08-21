use crate::{BuddyCollection, BuddyLine, OligarchyCollection};
use bitvec_crate::{prelude::BitArray, view::BitViewSized};

/// 用一个 `BitArray` 保存占用情况的伙伴行。
pub struct BitArrayBuddy<A: BitViewSized = [usize; 1]> {
    bits: BitArray<A>,
    base: usize,
    order: usize,
}

impl<A: BitViewSized> BitArrayBuddy<A> {
    /// 设置基序号。
    #[inline]
    pub fn set_base(&mut self, base: usize) {
        self.base = base;
    }
}

impl<A: BitViewSized> BuddyLine for BitArrayBuddy<A> {
    const MIN_ORDER: usize = 0;

    const EMPTY: Self = Self {
        bits: BitArray::ZERO,
        base: 0,
        order: 0,
    };

    #[inline]
    fn init(&mut self, order: usize, base: usize) {
        self.base = base;
        self.order = order;
    }

    #[inline]
    fn take(&mut self, idx: usize) -> bool {
        self.bits.replace(idx - self.base, false)
    }
}

impl<A: BitViewSized> OligarchyCollection for BitArrayBuddy<A> {
    fn take_any(&mut self, _align_order: usize, _count: usize) -> Option<usize> {
        todo!()
    }

    #[inline]
    fn put(&mut self, idx: usize) {
        self.bits.set(idx, true);
    }
}

impl<A: BitViewSized> BuddyCollection for BitArrayBuddy<A> {
    #[inline]
    fn take_any(&mut self, _align_order: usize) -> Option<usize> {
        if self.bits.is_empty() {
            None
        } else {
            let idx = self.bits.trailing_zeros();
            self.bits.set(idx, false);
            Some(idx)
        }
    }

    #[inline]
    fn put(&mut self, idx: usize) -> Option<usize> {
        let buddy = idx ^ 1;
        if self.bits.replace(buddy, false) {
            Some(idx << 1)
        } else {
            self.bits.set(idx, true);
            None
        }
    }
}
