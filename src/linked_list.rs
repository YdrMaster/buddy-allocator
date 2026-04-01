use crate::{BuddyCollection, BuddyLine, OligarchyCollection, Order};
use core::{fmt, ptr::NonNull};

/// 侵入式链表伙伴行。
///
/// 使用单向链表管理空闲内存块，适合管理小块内存。
/// 不支持对齐分配，时间复杂度为 O(n)。
pub struct LinkedListBuddy {
    /// 空闲链表头节点。
    free_list: Node,
    /// 当前阶数，用于指针和索引的转换。
    order: Order,
}

/// 必须实现 [`Send`] 才能加锁。
unsafe impl Send for LinkedListBuddy {}

impl BuddyLine for LinkedListBuddy {
    const INTRUSIVE_META_SIZE: usize = core::mem::size_of::<Node>();

    const EMPTY: Self = Self {
        free_list: Node { next: None },
        order: Order::new(0),
    };

    #[inline]
    fn init(&mut self, order: usize, _base: usize) {
        self.order = Order::new(order);
    }

    fn take(&mut self, _idx: usize) -> bool {
        todo!("not efficient")
    }
}

impl OligarchyCollection for LinkedListBuddy {
    #[inline]
    fn take_any(&mut self, align_order: usize, count: usize) -> Option<usize> {
        if count > 1 || align_order > 0 {
            // TODO 不支持
            None
        } else {
            // 直接从中删除一个节点
            self.free_list
                .take_any()
                .map(|ptr| self.order.ptr_to_idx(ptr))
        }
    }

    // 向头结点处插入一个节点
    #[inline]
    fn put(&mut self, idx: usize) {
        let ptr = self.order.idx_to_ptr(idx).expect("block address is null");
        self.free_list.insert_unordered(ptr);
    }
}

impl BuddyCollection for LinkedListBuddy {
    #[inline]
    fn take_any(&mut self, align_order: usize) -> Option<usize> {
        if align_order != 0 {
            // TODO 需要支持对齐吗？没效率，似乎没必要
            None
        } else {
            self.free_list
                .take_any()
                .map(|ptr| self.order.ptr_to_idx(ptr))
        }
    }

    // 向对应位置插入一个新的元素
    fn put(&mut self, idx: usize) -> Option<usize> {
        // 伙伴和当前结点存在链表的同一个位置。
        let node = self.order.idx_to_ptr(idx).expect("block address is null");
        // buddy序号为 0 时地址为空指针，不可能在空闲链表中，跳过合并。
        let Some(buddy) = self.order.idx_to_ptr(idx ^ 1) else {
            self.free_list.insert_unordered(node);
            return None;
        };
        if self.free_list.insert(node, buddy) {
            None
        } else {
            // 插入失败说明伙伴已碰头
            Some(idx >> 1)
        }
    }
}

impl fmt::Debug for LinkedListBuddy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut cursor = &self.free_list;
        while let Some(next) = cursor.next {
            write!(f, "{:#x}, ", self.order.ptr_to_idx(next))?;
            cursor = unsafe { next.as_ref() };
        }
        write!(f, "]")
    }
}

/// 链表节点。
///
/// 侵入式链表节点，直接存储在空闲内存块中。
#[repr(transparent)]
struct Node {
    /// 下一个节点的指针。
    next: Option<NonNull<Node>>,
}

impl Node {
    /// 插入结点，如果插入成功返回 `true`。
    /// 如果目标结点存在，则返回 `false`，且存在的结点也被移除。
    ///
    /// # Notice
    ///
    /// 这个函数可以尾递归的，但 Rust 并不支持优化尾递归。
    #[inline]
    fn insert(&mut self, mut node: NonNull<Node>, buddy: NonNull<Node>) -> bool {
        let mut cursor = self;
        loop {
            if let Some(mut next) = cursor.next {
                use core::cmp::Ordering::*;
                match next.cmp(&buddy) {
                    // 新结点更大，找下一个
                    Less => cursor = unsafe { next.as_mut() },
                    // 相等，移除这一个
                    Equal => {
                        cursor.next = unsafe { next.as_ref().next };
                        unsafe { node.as_mut() }.next = None;
                        break false;
                    }
                    // 新结点更小，插入
                    Greater => {
                        cursor.next = Some(node);
                        unsafe { node.as_mut() }.next = Some(next);
                        break true;
                    }
                }
            } else {
                // 没有下一个，插入
                cursor.next = Some(node);
                unsafe { node.as_mut() }.next = None;
                break true;
            }
        }
    }

    /// 直接在头结点插入。
    #[inline]
    fn insert_unordered(&mut self, mut node: NonNull<Node>) {
        unsafe { node.as_mut() }.next = self.next.replace(node);
    }

    /// 直接取下头结点。
    #[inline]
    fn take_any(&mut self) -> Option<NonNull<Node>> {
        let root = self.next;
        self.next = root.and_then(|node| unsafe { node.as_ref().next });
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 为测试分配一些内存作为节点存储
    #[repr(C, align(16))]
    struct TestMemory {
        data: [u8; 256],
    }

    #[test]
    fn test_empty() {
        let list = LinkedListBuddy::EMPTY;
        assert!(list.free_list.next.is_none());
    }

    #[test]
    fn test_init() {
        let mut list = LinkedListBuddy::EMPTY;
        list.init(3, 0);
        // order 应该被设置为 3
        assert_eq!(list.order.0, 3);
    }

    #[test]
    fn test_insert_unordered() {
        let mut list = LinkedListBuddy::EMPTY;
        list.init(4, 0); // order=4

        // 使用一个固定地址模拟节点
        let mut memory = TestMemory { data: [0; 256] };
        let node_ptr = NonNull::new(memory.data.as_mut_ptr().cast::<Node>()).unwrap();

        // 将指针转换为索引，然后 put
        let idx = list.order.ptr_to_idx(node_ptr);
        OligarchyCollection::put(&mut list, idx);

        // 链表不为空
        assert!(list.free_list.next.is_some());
    }

    #[test]
    fn test_take_and_put_buddy() {
        let mut list = LinkedListBuddy::EMPTY;
        list.init(4, 0); // order=4

        // 使用一个固定地址模拟节点
        let mut memory = TestMemory { data: [0; 256] };
        let node_ptr = NonNull::new(memory.data.as_mut_ptr().cast::<Node>()).unwrap();

        // 将指针转换为索引，然后 put
        let idx = list.order.ptr_to_idx(node_ptr);
        OligarchyCollection::put(&mut list, idx);

        // 可以成功取出
        let taken_idx = OligarchyCollection::take_any(&mut list, 0, 1);
        assert_eq!(taken_idx, Some(idx));
    }

    #[test]
    fn test_buddy_merge() {
        let mut list = LinkedListBuddy::EMPTY;
        list.init(4, 0); // order=4

        // 使用两个地址作为伙伴节点（地址差为 1<<4 = 16）
        let mut memory = TestMemory { data: [0; 256] };
        let base = memory.data.as_mut_ptr() as usize;
        // 确保 idx0 是偶数，这样 idx0 和 idx0^1 才是伙伴
        let ptr0 = (base + 15) & !15; // 对齐到 16

        // 转换为索引
        let idx0 = ptr0 >> 4;

        // 确保 idx0 是偶数
        let idx0 = idx0 & !1; // 清除最低位
        let idx1 = idx0 ^ 1; // 伙伴索引

        // 先放入 idx0
        OligarchyCollection::put(&mut list, idx0);
        // 再放入 idx1（伙伴是 idx0），应该会触发合并
        let result = BuddyCollection::put(&mut list, idx1);
        // 应该触发合并，返回父节点索引 (idx0 >> 1)
        assert_eq!(result, Some(idx0 >> 1));
    }

    #[test]
    fn test_buddy_collection_take_any() {
        let mut list = LinkedListBuddy::EMPTY;
        list.init(0, 0);

        // 空链表应该返回 None
        assert_eq!(BuddyCollection::take_any(&mut list, 0), None);

        // 不支持对齐
        assert_eq!(BuddyCollection::take_any(&mut list, 1), None);
    }

    #[test]
    fn test_oligarchy_collection_take_any() {
        let mut list = LinkedListBuddy::EMPTY;
        list.init(0, 0);

        // 不支持多个块
        assert_eq!(OligarchyCollection::take_any(&mut list, 0, 2), None);
        // 不支持对齐
        assert_eq!(OligarchyCollection::take_any(&mut list, 1, 1), None);
    }

    #[test]
    fn test_node_insert() {
        // 测试 Node::insert 方法
        let mut head = Node { next: None };

        let mut memory = TestMemory { data: [0; 256] };
        let ptr1 = NonNull::new(memory.data.as_mut_ptr().cast::<Node>()).unwrap();
        let ptr2 = NonNull::new(memory.data.as_mut_ptr().wrapping_add(16).cast::<Node>()).unwrap();
        let ptr3 = NonNull::new(memory.data.as_mut_ptr().wrapping_add(32).cast::<Node>()).unwrap();

        // 插入第一个节点
        head.insert_unordered(ptr1);
        assert!(head.next.is_some());

        // 插入第二个节点
        head.insert_unordered(ptr2);

        // 使用 insert 尝试找到伙伴（不存在的伙伴）
        let buddy = NonNull::new(memory.data.as_mut_ptr().wrapping_add(64).cast::<Node>()).unwrap();
        assert!(head.insert(ptr3, buddy));
    }

    #[test]
    fn test_node_take_any() {
        let mut head = Node { next: None };

        let mut memory = TestMemory { data: [0; 256] };
        let ptr = NonNull::new(memory.data.as_mut_ptr().cast::<Node>()).unwrap();

        head.insert_unordered(ptr);

        // 取出一个节点
        let taken = head.take_any();
        assert!(taken.is_some());

        // 再次取出应该返回 None
        assert!(head.take_any().is_none());
    }
}
