//! 伙伴分配器。

#![no_std]
#![deny(warnings, unstable_features, missing_docs)]

#[cfg(feature = "bitvec")]
mod bitvec;

#[cfg(feature = "bitvec")]
pub use bitvec::BitArrayBuddy;

use core::{alloc::Layout, num::NonZeroUsize, ops::Range, ptr::NonNull};

/// 伙伴分配器的一个行。
pub trait BuddyLine {
    /// 支持的最小阶数。
    ///
    /// 0 表示支持 1 字节的分配。
    const MIN_ORDER: usize;

    /// 空集合。用于静态初始化。
    const EMPTY: Self;

    /// 伙伴分配器可能需要集合知道自己的阶数和基序号。
    #[inline]
    fn init(&mut self, _order: usize, _base: usize) {}

    /// 提取指定位置的元素，返回是否提取到。
    #[inline]
    fn take(&mut self, _idx: usize) -> bool {
        unimplemented!()
    }
}

/// 寡头集合。伙伴分配器的顶层，不再合并。
pub trait OligarchyCollection: BuddyLine {
    /// 提取任何 `count` 个满足 `align_order` 的内存块。
    ///
    /// 返回提取到第一个元素的序号。找不到连续的那么多块，返回 [`None`]。
    fn take_any(&mut self, align_order: usize, count: usize) -> Option<usize>;

    /// 放入一个元素 `idx`。
    fn put(&mut self, idx: usize);
}

/// 伙伴集合。一组同阶的伙伴。
pub trait BuddyCollection: BuddyLine {
    /// 提取任何一个满足 `align_order` 的内存块。
    ///
    /// 返回提取到的元素。如果集合为空则无法提取，返回 [`None`]。
    fn take_any(&mut self, align_order: usize) -> Option<usize>;

    /// 放入一个元素 `idx`。
    ///
    /// 如果 `idx` 的伙伴元素存在，则两个元素都被提取并他们在上一层的序号。
    /// 否则 `idx` 被放入集合。
    fn put(&mut self, idx: usize) -> Option<usize>;
}

/// 伙伴分配器分配失败。
#[repr(transparent)]
pub struct BuddyError;

/// 伙伴分配器。
pub struct BuddyAllocator<const N: usize, O: OligarchyCollection, B: BuddyCollection> {
    /// 寡头集合。
    oligarchy: O,

    /// `N` 阶 `B` 型伙伴集合。
    buddies: [B; N],

    /// 最小阶数。
    ///
    /// `buddy[N - 1]` 伙伴集合分配内存块的阶数。
    /// 不能小于 `C::MIN_ORDER`。
    min_order: usize,
}

impl<const N: usize, O: OligarchyCollection, B: BuddyCollection> BuddyAllocator<N, O, B> {
    /// 最大层数。
    const MAX_LAYER: usize = N;

    /// 构造分配器。
    #[inline]
    pub const fn new() -> Self {
        Self {
            oligarchy: O::EMPTY,
            buddies: [B::EMPTY; N],
            min_order: 0,
        }
    }

    #[inline]
    const fn max_order(&self) -> usize {
        self.min_order + Self::MAX_LAYER
    }

    /// 运行时初始化。
    ///
    /// 设置每个伙伴集合的阶数。
    #[inline]
    pub fn init(&mut self, min_order: usize, base: NonNull<u8>) {
        self.min_order = min_order;
        let max_order = self.max_order();

        assert!(B::MIN_ORDER <= max_order);
        assert!(O::MIN_ORDER <= max_order);

        let base = base.as_ptr() as usize;
        self.buddies
            .iter_mut()
            .enumerate()
            .map(|(i, c)| (self.min_order + i, c))
            .for_each(|(o, c)| c.init(o, base >> o));
        self.oligarchy.init(max_order, base >> max_order);
    }

    /// 分配。
    ///
    /// 如果分配成功，返回一个能容纳 `layout` 的 `(指针, 长度)` 二元组。
    pub fn allocate(&mut self, layout: Layout) -> Result<(NonNull<u8>, usize), BuddyError> {
        #[inline]
        const fn allocated<T>(ptr: *mut T, size: usize) -> (NonNull<u8>, usize) {
            (unsafe { NonNull::new_unchecked(ptr) }.cast(), size)
        }

        // 支持零长分配
        if layout.size() == 0 {
            return Ok(allocated(self, 0));
        }
        // 容量的阶数
        let size = nonzero(layout.size());
        let size_order = if size.is_power_of_two() {
            size.trailing_zeros() as usize
        } else {
            // 向上取整
            (usize::BITS - size.leading_zeros()) as usize
        };
        let size_order = size_order.max(self.min_order);
        // 对齐的阶数
        let align_order = nonzero(layout.align()).trailing_zeros() as usize;
        let max_order = self.min_order + Self::MAX_LAYER;
        if size_order >= max_order {
            // 连续分配寡头
            match self
                .oligarchy
                .take_any(align_order >> max_order, 1 << (size_order - max_order))
            {
                Some(idx) => Ok(allocated((idx << max_order) as *mut (), 1 << size_order)),
                None => Err(BuddyError),
            }
        } else {
            let layer = size_order - self.min_order;
            match self.buddies[layer].take_any(align_order >> size_order) {
                // 一次分配成功
                Some(idx) => Ok(allocated((idx << size_order) as *mut (), 1 << size_order)),
                // 分配失败，需要上去借
                None => {
                    let mut ancestor = layer + 1;
                    let mut idx = loop {
                        // 从寡头借
                        if ancestor == Self::MAX_LAYER {
                            match self.oligarchy.take_any(align_order >> max_order, 1) {
                                Some(idx) => break idx,
                                None => Err(BuddyError)?,
                            }
                        }
                        // 从伙伴借
                        match self.buddies[ancestor]
                            .take_any(align_order >> (self.min_order + ancestor))
                        {
                            Some(idx) => break idx,
                            None => ancestor += 1,
                        }
                    };
                    // 多借的存回去
                    for layer in (layer..ancestor).rev() {
                        idx <<= 1;
                        assert!(self.buddies[layer].put(idx + 1).is_none());
                    }
                    // 完成
                    Ok(allocated((idx << size_order) as *mut (), 1 << size_order))
                }
            }
        }
    }

    /// 回收。
    pub fn deallocate(&mut self, range: Range<NonNull<u8>>) {
        let mut ptr = range.start.as_ptr() as usize;
        let end = range.end.as_ptr() as usize;
        let max_order = self.min_order + Self::MAX_LAYER;
        while ptr < end {
            // 剩余长度
            let len = nonzero(end - ptr);
            // 指针的对齐决定最大阶数
            let order_ptr = nonzero(ptr).trailing_zeros();
            // 长度向下取整也决定最大阶数
            let order_len = usize::BITS - len.leading_zeros() - 1;
            // 实际阶数是两个最大阶数中较小的那个
            let order = order_ptr.min(order_len) as usize;
            // 直接释放寡头
            if order >= max_order {
                // 寡头序号
                let idx = ptr >> max_order;
                // 寡头数量
                let count = len.get() >> max_order;
                // 移动指针
                ptr += count << max_order;
                // 释放
                (idx..).take(count).for_each(|idx| self.oligarchy.put(idx));
            } else {
                // 伙伴序号
                let mut idx = ptr >> order;
                // 移动指针
                ptr += 1 << order;
                // 释放
                for layer in (order - self.min_order).. {
                    // 释放寡头
                    if layer == Self::MAX_LAYER {
                        self.oligarchy.put(idx);
                        break;
                    }
                    // 释放伙伴
                    match self.buddies[layer].put(idx) {
                        Some(parent) => idx = parent,
                        None => break,
                    }
                }
            }
        }
    }
}

#[inline]
const fn nonzero(val: usize) -> NonZeroUsize {
    unsafe { NonZeroUsize::new_unchecked(val) }
}
