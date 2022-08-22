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
    fn swap(&mut self, idx: usize, val: bool) -> bool {
        let bit = 1usize << idx;
        let ans = self.bits & bit == bit;
        if val {
            self.bits |= bit;
        } else {
            self.bits &= !bit;
        }
        ans
    }
}

impl BuddyLine for UsizeBuddy {
    const EMPTY: Self = Self { bits: 0, base: 0 };

    #[inline]
    fn init(&mut self, order: usize, base: usize) {
        self.base = base;
    }

    #[inline]
    fn take(&mut self, idx: usize) -> bool {
        let idx = idx - self.base;
        debug_assert!(idx < Self::SIZE, "index out of bound");
        self.swap(idx, false)
    }
}

impl OligarchyCollection for UsizeBuddy {
    fn take_any(&mut self, align_order: usize, count: usize) -> Option<usize> {
        let align = (1usize << align_order) - 1;
        let mask = (1 << count) - 1;
        let mut skip = 0;
        loop {
            // 去掉已检查的部分
            let slice = self.bits >> skip;
            if slice == 0 {
                break;
            }
            // 跳过 0
            skip += (slice.trailing_zeros() as usize + align) & !align;
            if skip + count > Self::SIZE {
                break;
            }
            // 尝试占用
            if slice & mask == mask {
                self.bits |= mask << skip;
                return Some(self.base + skip);
            }
            // 跳过失败范围
            skip += (count + align) & !align;
            if skip + count > Self::SIZE {
                break;
            }
        }
        None
    }

    #[inline]
    fn put(&mut self, idx: usize) {
        let idx = idx - self.base;
        debug_assert!(idx < Self::SIZE, "index out of bound");
        self.swap(idx, true);
    }
}

impl BuddyCollection for UsizeBuddy {
    #[inline]
    fn take_any(&mut self, align_order: usize) -> Option<usize> {
        let align = (1usize << align_order) - 1;
        let mut skip = 0;
        loop {
            // 去掉已检查的部分
            let slice = self.bits >> skip;
            if slice == 0 {
                break;
            }
            // 对齐（bitvec 的 leading 是低位）
            skip += (slice.trailing_zeros() as usize + align) & !align;
            // 分配失败
            if skip >= Self::SIZE {
                break;
            }
            // 尝试占用
            if self.swap(skip, false) {
                return Some(self.base + skip);
            }
        }
        None
    }

    #[inline]
    fn put(&mut self, idx: usize) -> Option<usize> {
        let idx = idx - self.base;
        debug_assert!(idx < Self::SIZE, "index out of bound");
        let buddy = idx ^ 1;
        if self.swap(buddy, false) {
            Some(idx << 1)
        } else {
            self.swap(idx, true);
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
