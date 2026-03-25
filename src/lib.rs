//! 伙伴分配器。

#![no_std]
#![deny(warnings, unstable_features, missing_docs)]

mod avl;
mod bitmap;
mod linked_list;

pub use avl::AvlBuddy;
pub use bitmap::UsizeBuddy;
pub use linked_list::LinkedListBuddy;

use core::{alloc::Layout, fmt, num::NonZeroUsize, ptr::NonNull};

/// 伙伴分配器的一个行。
pub trait BuddyLine {
    /// 空集合。用于静态初始化。
    const EMPTY: Self;

    /// 侵入式元数据的大小。
    const INTRUSIVE_META_SIZE: usize = 0;

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
    /// 返回提取到第一个元素的序号。若找不到连续的那么多块，返回 [`None`]。
    fn take_any(&mut self, align_order: usize, count: usize) -> Option<usize>;

    /// 放入一个元素 `idx`。
    fn put(&mut self, idx: usize);
}

/// 伙伴集合。一组同阶的伙伴。
pub trait BuddyCollection: BuddyLine {
    /// 提取任何一个满足 `align_order` 的内存块。
    ///
    /// 返回提取到的元素。若集合为空则无法提取，返回 [`None`]。
    fn take_any(&mut self, align_order: usize) -> Option<usize>;

    /// 放入一个元素 `idx`。
    ///
    /// 如果 `idx` 的伙伴元素存在，则两个元素都被提取并返回他们在上一层的序号。
    /// 否则 `idx` 被放入集合。
    fn put(&mut self, idx: usize) -> Option<usize>;
}

/// 伙伴分配器分配失败。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct BuddyError;

/// 伙伴分配器。
pub struct BuddyAllocator<const N: usize, O: OligarchyCollection, B: BuddyCollection> {
    /// 寡头集合，管理最大阶数的内存块。
    oligarchy: O,

    /// `N` 阶 `B` 型伙伴集合。
    /// `buddies[i]` 管理阶数为 `min_order + i` 的内存块。
    buddies: [B; N],

    /// 最小阶数。
    ///
    /// `buddies[0]` 伙伴行分配的内存块的阶数。
    min_order: usize,

    /// 空闲容量（字节）。
    free: usize,

    /// 总容量（字节）。
    capacity: usize,
}

impl<const N: usize, O: OligarchyCollection, B: BuddyCollection> BuddyAllocator<N, O, B> {
    /// 最大层数。
    const MAX_LAYER: usize = N;
    /// 寡头支持的最小阶数。
    const O_MIN_ORDER: usize = O::INTRUSIVE_META_SIZE.next_power_of_two().trailing_zeros() as _;
    /// 伙伴支持的最小阶数。
    const B_MIN_ORDER: usize = B::INTRUSIVE_META_SIZE.next_power_of_two().trailing_zeros() as _;

    /// 构造分配器。
    #[inline]
    pub const fn new() -> Self {
        Self {
            oligarchy: O::EMPTY,
            buddies: [B::EMPTY; N],
            min_order: 0,
            free: 0,
            capacity: 0,
        }
    }
}

impl<const N: usize, O: OligarchyCollection, B: BuddyCollection> Default
    for BuddyAllocator<N, O, B>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize, O: OligarchyCollection, B: BuddyCollection> BuddyAllocator<N, O, B> {
    /// 返回分配器管理的总容量。
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 返回分配器剩余的空间容量。
    #[inline]
    pub fn free(&self) -> usize {
        self.free
    }

    /// 最大阶数。寡头块的阶数。
    #[inline]
    const fn max_order(&self) -> usize {
        self.min_order + Self::MAX_LAYER
    }

    /// 运行时初始化。
    ///
    /// 设置分配器分配的最小阶数和基址。
    #[inline]
    pub fn init<T>(&mut self, min_order: usize, base: NonNull<T>) {
        assert_eq!(
            0, self.capacity,
            "init is not allowed after any transfering"
        );

        self.min_order = min_order;
        let max_order = self.max_order();

        assert!(Self::O_MIN_ORDER <= max_order);
        assert!(Self::B_MIN_ORDER <= min_order);

        let base = base.as_ptr() as usize;
        self.buddies.iter_mut().enumerate().for_each(|(i, c)| {
            let o = self.min_order + i;
            c.init(o, base >> o)
        });
        self.oligarchy.init(max_order, base >> max_order);
    }

    /// 将一个 `ptr` 指向的长度为 `usize` 的内存块转移给分配器。
    ///
    /// # Safety
    ///
    /// 调用者需要保证：
    ///
    /// - 这个内存块没有被其他任何对象引用；
    /// - 这个内存块和已经托管的内存块不重叠。
    #[inline]
    pub unsafe fn transfer<T>(&mut self, ptr: NonNull<T>, size: usize) {
        self.capacity += size;
        self.deallocate(ptr, size)
    }

    /// 从分配器夺走一个对齐到 `align_order` 阶，长度为 `size` 的内存块。
    #[inline]
    pub fn snatch<T>(
        &mut self,
        align_order: usize,
        size: NonZeroUsize,
    ) -> Result<(NonNull<T>, usize), BuddyError> {
        let ans = self.allocate(align_order, size);
        if let Ok((_, size)) = ans {
            self.capacity -= size;
        }
        ans
    }

    /// 分配可容纳 `T` 对象的内存块。
    #[inline]
    pub fn allocate_type<T>(&mut self) -> Result<(NonNull<T>, usize), BuddyError> {
        self.allocate_layout(Layout::new::<T>())
    }

    /// 分配符合 `layout` 布局的内存块。
    #[inline]
    pub fn allocate_layout<T>(
        &mut self,
        layout: Layout,
    ) -> Result<(NonNull<T>, usize), BuddyError> {
        #[inline]
        const fn allocated<T, U>(ptr: *mut T, size: usize) -> (NonNull<U>, usize) {
            (unsafe { NonNull::new_unchecked(ptr) }.cast(), size)
        }

        if let Some(size) = NonZeroUsize::new(layout.size()) {
            self.allocate(layout.align().trailing_zeros() as _, size)
        } else {
            Ok(allocated(self, 0))
        }
    }

    /// 分配。
    ///
    /// 如果分配成功，返回一个 `(指针, 长度)` 二元组。
    pub fn allocate<T>(
        &mut self,
        align_order: usize,
        size: NonZeroUsize,
    ) -> Result<(NonNull<T>, usize), BuddyError> {
        let max_order = self.max_order();
        #[inline]
        const fn allocated<T, U>(ptr: *mut T, size: usize) -> (NonNull<U>, usize) {
            (unsafe { NonNull::new_unchecked(ptr) }.cast(), size)
        }

        // 要分配的容量
        let page_mask = (1usize << self.min_order) - 1;
        let ans_size = (size.get() + page_mask) & !page_mask;
        // 分配的阶数
        let size_order = nonzero(ans_size.next_power_of_two()).trailing_zeros() as usize;
        // 分配
        let (ptr, alloc_size) = if size_order >= max_order {
            // 连续分配寡头
            let count = ((ans_size >> (max_order - 1)) + 1) >> 1;
            let align_offset = align_order.saturating_sub(max_order);
            match self.oligarchy.take_any(align_offset, count) {
                Some(idx) => (idx << max_order, count << max_order),
                None => Err(BuddyError)?,
            }
        } else {
            // 分配伙伴
            let layer0 = size_order - self.min_order;
            let mut layer = layer0;
            let mut idx = loop {
                // 从寡头借
                if layer == Self::MAX_LAYER {
                    let align_offset = align_order.saturating_sub(max_order);
                    match self.oligarchy.take_any(align_offset, 1) {
                        Some(idx) => break idx,
                        None => Err(BuddyError)?,
                    }
                }
                // 从伙伴借
                let align_offset = align_order.saturating_sub(self.min_order + layer);
                match self.buddies[layer].take_any(align_offset) {
                    Some(idx) => break idx,
                    None => layer += 1,
                }
            };
            // 存回多借用的
            assert!(self.buddies[layer0..layer].iter_mut().rev().all(|b| {
                idx <<= 1;
                b.put(idx + 1).is_none()
            }));
            // 完成
            (idx << size_order, 1 << size_order)
        };
        self.free -= alloc_size;
        // 存回为了对齐而多分配的
        if alloc_size > ans_size {
            self.deallocate(
                unsafe { NonNull::new_unchecked((ptr + ans_size) as *mut u8) },
                alloc_size - ans_size,
            );
        }
        Ok(allocated(ptr as *mut (), ans_size))
    }

    /// 根据布局回收。
    ///
    /// # Safety
    ///
    /// 这个方法认为 `ptr` 是根据 `layout` 分配出来的，
    /// 因此长度不小于 `layout.size()` 并且对齐到 `self.min_order`。
    pub unsafe fn deallocate_layout<T>(&mut self, ptr: NonNull<T>, layout: Layout) {
        debug_assert!((1 << (ptr.as_ptr() as usize).trailing_zeros()) >= layout.align());

        let mask = (1 << self.min_order) - 1;
        self.deallocate(ptr, (layout.size() + mask) & !mask)
    }

    /// 回收。
    ///
    /// # Notice
    ///
    /// 调用者需要保证 `size` 对齐了分配器的最小阶数。
    pub fn deallocate<T>(&mut self, ptr: NonNull<T>, size: usize) {
        debug_assert!(
            size.trailing_zeros() as usize >= self.min_order,
            "size must align to minium order"
        );

        let max_order = self.max_order();

        let mut ptr = ptr.as_ptr() as usize;
        let end = ptr + size;
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
        self.free += size;
        assert!(
            self.free <= self.capacity,
            "something wrong with the free bytes, it is larger than the capacity: {} > {}",
            self.free,
            self.capacity
        );
    }
}

impl<const N: usize, O: OligarchyCollection + fmt::Debug, B: BuddyCollection + fmt::Debug>
    fmt::Debug for BuddyAllocator<N, O, B>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BuddyAllocator@{:#018x}", self as *const _ as usize)?;
        writeln!(f, "---------------------------------")?;
        for (i, line) in self.buddies.iter().enumerate() {
            writeln!(f, "{:>2}> {line:?}", self.min_order + i)?;
        }
        writeln!(f, "{:>2}> {:?}", self.max_order(), self.oligarchy)
    }
}

#[inline]
const fn nonzero(val: usize) -> NonZeroUsize {
    unsafe { NonZeroUsize::new_unchecked(val) }
}

/// 阶数。
///
/// 用于侵入式行序号到指针的转换。
struct Order(usize);

impl Order {
    #[inline]
    const fn new(order: usize) -> Self {
        Self(order)
    }

    #[inline]
    unsafe fn idx_to_ptr<T>(&self, idx: usize) -> NonNull<T> {
        unsafe { NonNull::new_unchecked((idx << self.0) as *mut _) }
    }

    #[inline]
    fn ptr_to_idx<T>(&self, ptr: NonNull<T>) -> usize {
        (ptr.as_ptr() as usize) >> self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LinkedListBuddy, UsizeBuddy};

    // 定义测试用的分配器类型
    type TestAllocator<const N: usize> = BuddyAllocator<N, UsizeBuddy, LinkedListBuddy>;

    /// 测试内存区域，用于模拟堆内存
    #[repr(C, align(4096))]
    #[derive(Clone, Copy)]
    struct TestPage([u8; 4096]);

    static mut TEST_MEMORY: [TestPage; 16] = [TestPage([0; 4096]); 16];

    #[test]
    fn test_allocator_new() {
        let allocator: TestAllocator<4> = BuddyAllocator::new();
        assert_eq!(allocator.capacity(), 0);
        assert_eq!(allocator.free(), 0);
        assert_eq!(allocator.min_order, 0);
    }

    #[test]
    fn test_allocator_default() {
        let allocator: TestAllocator<4> = BuddyAllocator::default();
        assert_eq!(allocator.capacity(), 0);
        assert_eq!(allocator.free(), 0);
    }

    #[test]
    fn test_allocator_init() {
        let mut allocator: TestAllocator<4> = BuddyAllocator::new();

        // 获取测试内存地址
        let ptr = NonNull::new(core::ptr::addr_of_mut!(TEST_MEMORY).cast::<u8>()).unwrap();

        allocator.init(12, ptr);
        assert_eq!(allocator.min_order, 12);
        assert_eq!(allocator.capacity(), 0);
        assert_eq!(allocator.free(), 0);
    }

    #[test]
    fn test_allocator_transfer() {
        let mut allocator: TestAllocator<4> = BuddyAllocator::new();

        let ptr = NonNull::new(core::ptr::addr_of_mut!(TEST_MEMORY).cast::<u8>()).unwrap();
        let len = core::mem::size_of_val(unsafe { &*core::ptr::addr_of!(TEST_MEMORY) });

        allocator.init(12, ptr);

        // 转移内存给分配器
        unsafe {
            allocator.transfer(ptr, len);
        }

        assert_eq!(allocator.capacity(), len);
        assert_eq!(allocator.free(), len);
    }

    #[test]
    fn test_allocator_allocate_deallocate_basic() {
        let mut allocator: TestAllocator<8> = BuddyAllocator::new();

        let ptr = NonNull::new(core::ptr::addr_of_mut!(TEST_MEMORY).cast::<u8>()).unwrap();
        let len = core::mem::size_of_val(unsafe { &*core::ptr::addr_of!(TEST_MEMORY) });

        allocator.init(12, ptr);
        unsafe {
            allocator.transfer(ptr, len);
        }

        let initial_free = allocator.free();

        // 分配一个 4KB 的块
        let size = NonZeroUsize::new(4096).unwrap();
        let (alloc_ptr, alloc_size) = allocator.allocate::<u8>(0, size).unwrap();

        // 验证分配成功
        assert!(alloc_size >= 4096);
        assert!(allocator.free() < initial_free);

        // 释放内存
        allocator.deallocate(alloc_ptr, alloc_size);

        // 验证释放后空闲空间恢复
        assert_eq!(allocator.free(), initial_free);
    }

    #[test]
    fn test_allocator_allocate_type() {
        let mut allocator: TestAllocator<8> = BuddyAllocator::new();

        let ptr = NonNull::new(core::ptr::addr_of_mut!(TEST_MEMORY).cast::<u8>()).unwrap();
        let len = core::mem::size_of_val(unsafe { &*core::ptr::addr_of!(TEST_MEMORY) });

        allocator.init(12, ptr);
        unsafe {
            allocator.transfer(ptr, len);
        }

        let initial_free = allocator.free();

        // 分配一个 usize 大小的内存
        let (alloc_ptr, alloc_size) = allocator.allocate_type::<usize>().unwrap();

        // 验证分配成功
        assert!(alloc_size >= core::mem::size_of::<usize>());

        // 释放内存
        allocator.deallocate(alloc_ptr, alloc_size);

        assert_eq!(allocator.free(), initial_free);
    }

    #[test]
    fn test_allocator_allocate_layout() {
        let mut allocator: TestAllocator<8> = BuddyAllocator::new();

        let ptr = NonNull::new(core::ptr::addr_of_mut!(TEST_MEMORY).cast::<u8>()).unwrap();
        let len = core::mem::size_of_val(unsafe { &*core::ptr::addr_of!(TEST_MEMORY) });

        allocator.init(12, ptr);
        unsafe {
            allocator.transfer(ptr, len);
        }

        let initial_free = allocator.free();

        // 创建一个对齐要求为 8 的 layout
        let layout = Layout::from_size_align(64, 8).unwrap();
        let (alloc_ptr, alloc_size) = allocator.allocate_layout::<u8>(layout).unwrap();

        // 验证分配成功
        assert!(alloc_size >= 64);

        // 释放内存
        unsafe {
            allocator.deallocate_layout(alloc_ptr, layout);
        }

        assert_eq!(allocator.free(), initial_free);
    }

    #[test]
    fn test_allocator_multiple_allocations() {
        let mut allocator: TestAllocator<8> = BuddyAllocator::new();

        let ptr = NonNull::new(core::ptr::addr_of_mut!(TEST_MEMORY).cast::<u8>()).unwrap();
        let len = core::mem::size_of_val(unsafe { &*core::ptr::addr_of!(TEST_MEMORY) });

        allocator.init(12, ptr);
        unsafe {
            allocator.transfer(ptr, len);
        }

        let initial_free = allocator.free();

        // 分配多个内存块
        let size1 = NonZeroUsize::new(1024).unwrap();
        let size2 = NonZeroUsize::new(2048).unwrap();
        let size3 = NonZeroUsize::new(4096).unwrap();

        let (ptr1, size1) = allocator.allocate::<u8>(0, size1).unwrap();
        let (ptr2, size2) = allocator.allocate::<u8>(0, size2).unwrap();
        let (ptr3, size3) = allocator.allocate::<u8>(0, size3).unwrap();

        // 释放其中一个
        allocator.deallocate(ptr2, size2);

        // 释放剩余
        allocator.deallocate(ptr1, size1);
        allocator.deallocate(ptr3, size3);

        assert_eq!(allocator.free(), initial_free);
    }

    #[test]
    fn test_allocator_snatch() {
        let mut allocator: TestAllocator<8> = BuddyAllocator::new();

        let ptr = NonNull::new(core::ptr::addr_of_mut!(TEST_MEMORY).cast::<u8>()).unwrap();
        let len = core::mem::size_of_val(unsafe { &*core::ptr::addr_of!(TEST_MEMORY) });

        allocator.init(12, ptr);
        unsafe {
            allocator.transfer(ptr, len);
        }

        let initial_capacity = allocator.capacity();
        let initial_free = allocator.free();

        // 使用 snatch 分配内存，这会减少 capacity
        let size = NonZeroUsize::new(4096).unwrap();
        let (_alloc_ptr, alloc_size) = allocator.snatch::<u8>(0, size).unwrap();

        // snatch 会减少 capacity 和 free
        assert!(allocator.capacity() < initial_capacity);
        assert_eq!(allocator.capacity(), initial_capacity - alloc_size);
        assert_eq!(allocator.free(), initial_free - alloc_size);

        // 注意：snatch 后不应该调用 deallocate，因为这会导致 free > capacity
        // 这是 snatch 的语义：永久夺取内存，不再归还
    }

    #[test]
    fn test_allocator_allocate_failure() {
        let mut allocator: TestAllocator<8> = BuddyAllocator::new();

        let ptr = NonNull::new(core::ptr::addr_of_mut!(TEST_MEMORY).cast::<u8>()).unwrap();
        let len = core::mem::size_of_val(unsafe { &*core::ptr::addr_of!(TEST_MEMORY) });

        allocator.init(12, ptr);
        unsafe {
            allocator.transfer(ptr, len);
        }

        // 尝试分配一个超过容量的块（应该失败）
        // 使用一个合理的大值，避免溢出
        let huge_size = NonZeroUsize::new(len * 2).unwrap();
        assert!(allocator.allocate::<u8>(0, huge_size).is_err());
    }

    #[test]
    fn test_allocator_zero_size_allocation() {
        let mut allocator: TestAllocator<8> = BuddyAllocator::new();

        let ptr = NonNull::new(core::ptr::addr_of_mut!(TEST_MEMORY).cast::<u8>()).unwrap();
        let len = core::mem::size_of_val(unsafe { &*core::ptr::addr_of!(TEST_MEMORY) });

        allocator.init(12, ptr);
        unsafe {
            allocator.transfer(ptr, len);
        }

        // 使用 allocate_layout 分配零大小
        let layout = Layout::from_size_align(0, 1).unwrap();
        let (_alloc_ptr, alloc_size) = allocator.allocate_layout::<u8>(layout).unwrap();

        // 零大小分配应该返回非空指针但大小为 0
        assert_eq!(alloc_size, 0);
    }

    #[test]
    fn test_max_order() {
        let mut allocator: TestAllocator<4> = BuddyAllocator::new();

        let ptr = NonNull::new(core::ptr::addr_of_mut!(TEST_MEMORY).cast::<u8>()).unwrap();

        allocator.init(3, ptr);

        // max_order = min_order + MAX_LAYER = 3 + 4 = 7
        assert_eq!(allocator.max_order(), 7);
    }

    #[test]
    fn test_order_idx_to_ptr() {
        let order = Order::new(12);

        // 测试 idx 到 ptr 的转换
        // idx=0 会生成空指针，所以从 1 开始
        let ptr = unsafe { order.idx_to_ptr::<u8>(1) };
        assert_eq!(ptr.as_ptr() as usize, 4096); // 1 << 12

        let ptr = unsafe { order.idx_to_ptr::<u8>(2) };
        assert_eq!(ptr.as_ptr() as usize, 8192); // 2 << 12

        let ptr = unsafe { order.idx_to_ptr::<u8>(3) };
        assert_eq!(ptr.as_ptr() as usize, 12288); // 3 << 12
    }

    #[test]
    fn test_order_ptr_to_idx() {
        let order = Order::new(12);

        // 测试 ptr 到 idx 的转换
        let ptr = NonNull::new(4096 as *mut u8).unwrap();
        assert_eq!(order.ptr_to_idx(ptr), 1);

        let ptr = NonNull::new(8192 as *mut u8).unwrap();
        assert_eq!(order.ptr_to_idx(ptr), 2);
    }

    #[test]
    fn test_order_roundtrip() {
        let order = Order::new(12);

        // 测试 idx -> ptr -> idx 的往返转换
        // idx=0 会生成空指针，所以从 1 开始
        for i in [1, 5, 100, 1000] {
            let ptr = unsafe { order.idx_to_ptr::<u8>(i) };
            let idx = order.ptr_to_idx(ptr);
            assert_eq!(idx, i);
        }
    }
}
