use crate::{BuddyCollection, BuddyLine, OligarchyCollection};
use bitvec_crate::{prelude::BitArray, view::BitViewSized};
use core::fmt;

/// 用一个 `BitArray` 保存占用情况的伙伴行。
///
/// - 非侵入式
/// - 静态分配，容量有限
pub struct BitArrayBuddy<A: BitViewSized = usize> {
    bits: BitArray<A>,
    base: usize,
    order: usize,
}

impl<A: BitViewSized> BitArrayBuddy<A> {
    #[inline]
    fn swap_unchecked(&mut self, idx: usize, val: bool) -> bool {
        unsafe { self.bits.replace_unchecked(idx, val) }
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
        let idx = idx - self.base;
        assert!(idx < self.bits.len(), "index out of bound");
        self.swap_unchecked(idx, false)
    }
}

impl<A: BitViewSized> OligarchyCollection for BitArrayBuddy<A> {
    fn take_any(&mut self, align_order: usize, count: usize) -> Option<usize> {
        let mask = (1usize << align_order) - 1;
        let mut skip = 0;
        loop {
            // 去掉已检查的部分
            let slice = &self.bits[skip..];
            if !*unsafe { slice.get_unchecked(0) } {
                // 已经判断过 0 号位不是 0，返回 0 表示没有 1
                let zeros = slice.leading_zeros();
                if zeros == 0 {
                    break;
                }
                // 跳过 0
                skip += (zeros + mask) & !mask;
                if skip + count > self.bits.len() {
                    break;
                }
            }
            // 尝试占用
            let slice = &mut self.bits[skip..][..count];
            if slice.all() {
                slice.fill(false);
                return Some(self.base + skip);
            } else {
                // 跳过失败范围
                skip += (count + mask) & !mask;
                if skip + count > self.bits.len() {
                    break;
                }
            }
        }
        None
    }

    #[inline]
    fn put(&mut self, idx: usize) {
        let idx = idx - self.base;
        assert!(idx < self.bits.len(), "index out of bound");
        self.swap_unchecked(idx, true);
    }
}

impl<A: BitViewSized> BuddyCollection for BitArrayBuddy<A> {
    #[inline]
    fn take_any(&mut self, align_order: usize) -> Option<usize> {
        let mask = (1usize << align_order) - 1;
        let mut skip = 0;
        loop {
            // 去掉已检查的部分
            let slice = &self.bits[skip..];
            if slice.is_empty() {
                break;
            }
            // 对齐（bitvec 的 leading 是低位）
            skip += (slice.leading_zeros() + mask) & !mask;
            // 分配失败
            if skip >= self.bits.len() {
                break;
            }
            // 尝试占用
            if self.swap_unchecked(skip, false) {
                return Some(self.base + skip);
            }
        }
        None
    }

    #[inline]
    fn put(&mut self, idx: usize) -> Option<usize> {
        let idx = idx - self.base;
        assert!(idx < self.bits.len(), "index out of bound");
        let buddy = idx ^ 1;
        if self.swap_unchecked(buddy, false) {
            Some(idx << 1)
        } else {
            self.swap_unchecked(idx, true);
            None
        }
    }
}

impl<A: BitViewSized> fmt::Debug for BitArrayBuddy<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        '['.fmt(f)?;
        for i in self.bits.iter_ones() {
            write!(f, "{}, ", self.base + i)?;
        }
        ']'.fmt(f)
    }
}
