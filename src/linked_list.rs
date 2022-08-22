use crate::{BuddyCollection, BuddyLine, Intrusive};
use core::{cmp::Ordering::*, fmt, ptr::NonNull};

/// 侵入式链表伙伴行。
pub struct LinkedListBuddy {
    free_list: Node,
    intrusive: Intrusive,
}

impl BuddyLine for LinkedListBuddy {
    const INTRUSIVE_META_SIZE: usize = core::mem::size_of::<Node>();

    const EMPTY: Self = Self {
        free_list: Node { next: None },
        intrusive: Intrusive::ZERO,
    };

    #[inline]
    fn init(&mut self, order: usize, _base: usize) {
        self.intrusive.init(order)
    }

    fn take(&mut self, _idx: usize) -> bool {
        // TODO 可以实现，但没有效率
        unimplemented!("not efficient")
    }
}

// TODO 可以实现，但没有效率
// impl OligarchyCollection for LinkedListBuddy {
//     fn take_any(&mut self, _align_order: usize, _count: usize) -> Option<usize> {
//         todo!()
//     }

//     fn put(&mut self, _idx: usize) {
//         todo!()
//     }
// }

impl BuddyCollection for LinkedListBuddy {
    #[inline]
    fn take_any(&mut self, align_order: usize) -> Option<usize> {
        if align_order != 0 {
            // TODO 需要支持对齐吗？没效率，似乎没必要
            None
        } else {
            self.free_list
                .take_any()
                .map(|ptr| unsafe { self.intrusive.ptr_to_idx(ptr) })
        }
    }

    fn put(&mut self, idx: usize) -> Option<usize> {
        // 伙伴和当前结点存在链表的同一个位置。
        let node = unsafe { self.intrusive.idx_to_ptr(idx) };
        let buddy = unsafe { self.intrusive.idx_to_ptr(idx ^ 1) };
        if self.free_list.insert(node, buddy) {
            None
        } else {
            // 插入失败说明伙伴已碰头
            Some(idx >> 1)
        }
    }
}

impl fmt::Debug for LinkedListBuddy {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[repr(transparent)]
struct Node {
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

    /// 直接取下头结点。
    #[inline]
    fn take_any(&mut self) -> Option<NonNull<Node>> {
        let root = self.next.take();
        if let Some(root) = root {
            self.next = unsafe { root.as_ref().next };
        }
        root
    }
}
