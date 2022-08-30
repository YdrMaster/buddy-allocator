use crate::{BuddyCollection, BuddyLine, OligarchyCollection, Order};
use core::{fmt, ptr::NonNull};

/// 侵入式链表伙伴行。
pub struct LinkedListBuddy {
    free_list: Node,
    order: Order,
}

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
        // TODO 可以实现，但没有效率
        unimplemented!("not efficient")
    }
}

impl OligarchyCollection for LinkedListBuddy {
    #[inline]
    fn take_any(&mut self, align_order: usize, count: usize) -> Option<usize> {
        if count > 1 || align_order > 0 {
            // TODO 不支持
            None
        } else {
            self.free_list
                .take_any()
                .map(|ptr| self.order.ptr_to_idx(ptr))
        }
    }

    #[inline]
    fn put(&mut self, idx: usize) {
        self.free_list
            .insert_unordered(unsafe { self.order.idx_to_ptr(idx) });
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

    fn put(&mut self, idx: usize) -> Option<usize> {
        // 伙伴和当前结点存在链表的同一个位置。
        let node = unsafe { self.order.idx_to_ptr(idx) };
        let buddy = unsafe { self.order.idx_to_ptr(idx ^ 1) };
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
            self.order.ptr_to_idx(next).fmt(f)?;
            write!(f, ", ")?;
            cursor = unsafe { next.as_ref() };
        }
        write!(f, "]")
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
        unsafe { node.as_mut() }.next = core::mem::replace(&mut self.next, Some(node));
    }

    /// 直接取下头结点。
    #[inline]
    fn take_any(&mut self) -> Option<NonNull<Node>> {
        let root = self.next;
        self.next = root.and_then(|node| unsafe { node.as_ref().next });
        root
    }
}
