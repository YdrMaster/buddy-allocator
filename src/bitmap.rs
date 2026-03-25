use crate::{BuddyCollection, BuddyLine, OligarchyCollection};
use core::fmt;

/// 用一个 usize 作为位图保存占用情况的伙伴行。
///
/// - 非侵入式
/// - 静态分配，容量有限（最多 64 或 128 个块，取决于平台）
/// - 查找和插入时间复杂度为 O(1)
pub struct UsizeBuddy {
    /// 位图，1 表示空闲，0 表示已分配。
    bits: usize,
    /// 基序号，用于将本地索引转换为全局索引。
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
        if count == 0 {
            return None;
        }
        if count == 1 {
            // 单个块，直接使用 BuddyCollection 的逻辑
            return BuddyCollection::take_any(self, align_order);
        }

        // 需要找到连续的 count 个位
        // mask 是 count 个连续的 1
        let mask = (1usize << count) - 1;
        let align = 1usize << align_order;
        let mut i = 0;
        while i + count <= usize::BITS as usize {
            let bits_mask = mask << i;
            if self.bits & bits_mask == bits_mask {
                self.bits &= !bits_mask;
                return Some(self.base + i);
            }
            i += align;
        }
        None
    }

    #[inline]
    fn put(&mut self, idx: usize) {
        self.bits |= 1 << (idx - self.base);
    }
}

impl BuddyCollection for UsizeBuddy {
    #[inline]
    fn take_any(&mut self, align_order: usize) -> Option<usize> {
        // 将位图对齐到指定对齐阶数
        // align_order=1 要求索引是 2 的倍数（0, 2, 4...）
        // 需要清除 bit 0, 1, ..., align_order-1 中不满足对齐的位
        // 正确做法：保留每 2^align_order 个位中的第一个
        if align_order == 0 {
            // 不对齐，直接找第一个空闲位
            if self.bits != 0 {
                let i = self.bits.trailing_zeros() as usize;
                self.bits &= !(1 << i);
                return Some(self.base + i);
            }
        } else {
            // 对齐：只保留 bit 0, 2^align_order, 2*2^align_order, ...
            let align = 1usize << align_order;
            // 创建掩码，只保留对齐的位
            let mut mask = 0usize;
            for i in (0..usize::BITS as usize).step_by(align) {
                mask |= 1 << i;
            }
            let aligned_bits = self.bits & mask;
            if aligned_bits != 0 {
                let i = aligned_bits.trailing_zeros() as usize;
                self.bits &= !(1 << i);
                return Some(self.base + i);
            }
        }
        None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buddy_line_init() {
        let mut buddy = UsizeBuddy::EMPTY;
        buddy.init(3, 10);
        assert_eq!(buddy.base, 10);
    }

    #[test]
    fn test_take_any_basic() {
        let mut buddy = UsizeBuddy {
            bits: 0b1010, // 位 1 和 3 是空闲的
            base: 0,
        };

        // 应该返回最小的空闲位（1）
        assert_eq!(BuddyCollection::take_any(&mut buddy, 0), Some(1));
        // 现在位 3 是空闲的
        assert_eq!(BuddyCollection::take_any(&mut buddy, 0), Some(3));
        // 没有更多空闲位
        assert_eq!(BuddyCollection::take_any(&mut buddy, 0), None);
    }

    #[test]
    fn test_take_any_with_align() {
        let mut buddy = UsizeBuddy {
            bits: 0b1111, // 位 0,1,2,3 都是空闲的
            base: 0,
        };

        // align_order=1 要求索引对齐到 2（即索引必须是 0,2,4...）
        // 应该返回 0
        assert_eq!(BuddyCollection::take_any(&mut buddy, 1), Some(0));
        // 现在位 1,2,3 是空闲的，对齐到 2 的最小是 2
        assert_eq!(BuddyCollection::take_any(&mut buddy, 1), Some(2));
        // 没有更多对齐到 2 的空闲位
        assert_eq!(BuddyCollection::take_any(&mut buddy, 1), None);
    }

    #[test]
    fn test_put_basic() {
        let mut buddy = UsizeBuddy {
            bits: 0b0000,
            base: 0,
        };

        // 放入索引 0
        assert_eq!(BuddyCollection::put(&mut buddy, 0), None);
        assert_eq!(buddy.bits, 0b0001);

        // 放入索引 2
        assert_eq!(BuddyCollection::put(&mut buddy, 2), None);
        assert_eq!(buddy.bits, 0b0101);
    }

    #[test]
    fn test_put_merge_buddy() {
        let mut buddy = UsizeBuddy {
            bits: 0b0001, // 本地索引 0 是空闲的
            base: 0,
        };

        // 放入全局索引 1（本地索引 1），伙伴本地索引 0 存在，触发合并
        // 返回父节点的本地索引（idx << 1 格式，调用者需要 idx >> 1 得到实际父节点）
        assert_eq!(BuddyCollection::put(&mut buddy, 1), Some(2));
        assert_eq!(buddy.bits, 0b0000);
    }

    #[test]
    fn test_put_merge_buddy_base_offset() {
        // 测试 base 不为 0 的情况
        let mut buddy = UsizeBuddy {
            bits: 0b0000, // 初始为空
            base: 10,
        };

        // 放入全局索引 10（本地索引 0），伙伴不存在
        assert_eq!(BuddyCollection::put(&mut buddy, 10), None);
        assert_eq!(buddy.bits, 0b0001);

        // 放入全局索引 11（本地索引 1），伙伴存在，触发合并
        assert_eq!(BuddyCollection::put(&mut buddy, 11), Some(2));
        assert_eq!(buddy.bits, 0b0000);
    }

    #[test]
    fn test_take_by_index() {
        let mut buddy = UsizeBuddy {
            bits: 0b1010,
            base: 0,
        };

        // 提取索引 3
        assert!(buddy.take(3));
        assert_eq!(buddy.bits, 0b0010);

        // 再次提取索引 3（应该失败）
        assert!(!buddy.take(3));

        // 提取索引 1
        assert!(buddy.take(1));
        assert_eq!(buddy.bits, 0b0000);
    }

    #[test]
    fn test_oligarchy_take_any_single() {
        let mut buddy = UsizeBuddy {
            bits: 0b1010,
            base: 0,
        };

        // count=1 应该使用 BuddyCollection 的逻辑
        assert_eq!(OligarchyCollection::take_any(&mut buddy, 0, 1), Some(1));
        assert_eq!(OligarchyCollection::take_any(&mut buddy, 0, 1), Some(3));
        assert_eq!(OligarchyCollection::take_any(&mut buddy, 0, 1), None);
    }

    #[test]
    fn test_oligarchy_take_any_multiple() {
        let mut buddy = UsizeBuddy {
            bits: 0b0111, // 位 0,1,2 是空闲的
            base: 0,
        };

        // 取 2 个连续的位
        assert_eq!(OligarchyCollection::take_any(&mut buddy, 0, 2), Some(0));
        // 位 0,1 被取走，剩下位 2
        assert_eq!(buddy.bits, 0b0100);

        // 再尝试取 2 个连续的位（失败）
        assert_eq!(OligarchyCollection::take_any(&mut buddy, 0, 2), None);
    }

    #[test]
    fn test_oligarchy_take_any_count_zero() {
        let mut buddy = UsizeBuddy {
            bits: 0b1111,
            base: 0,
        };

        // count=0 应该返回 None
        assert_eq!(OligarchyCollection::take_any(&mut buddy, 0, 0), None);
        assert_eq!(buddy.bits, 0b1111); // 位图不变
    }

    #[test]
    fn test_oligarchy_take_any_with_align() {
        let mut buddy = UsizeBuddy {
            bits: 0b111111, // 位 0-5 是空闲的
            base: 0,
        };

        // 取 2 个连续的位，align_order=1（对齐到 2）
        // 可能的起始位置是 0,2,4
        // 0-1 都空闲，所以可以取
        assert_eq!(OligarchyCollection::take_any(&mut buddy, 1, 2), Some(0));
        // 位 0,1 被取走
        assert_eq!(buddy.bits, 0b111100);

        // 再取 2 个连续的位，对齐到 2
        // 可能的起始位置是 2,4
        // 都可以取
        assert_eq!(OligarchyCollection::take_any(&mut buddy, 1, 2), Some(2));
        assert_eq!(buddy.bits, 0b110000);
    }

    #[test]
    fn test_empty() {
        let buddy = UsizeBuddy::EMPTY;
        assert_eq!(buddy.bits, 0);
        assert_eq!(buddy.base, 0);
    }
}
