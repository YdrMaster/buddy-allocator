// # 静止的 AVL 树左右子树的高度差的绝对值最大为 1
//
// 如果如下的树是 AVL 树：
//
//     A
//    / \
//   B   γ
//  / \
// α   β
//
// 则 | α - β | <= 1, | max(α, β) + 1 - γ | <= 1。
//
// 如果这是一个需要右单旋的情况（α - β = 1, B - γ = 2），设 x = β，显然：
//
// - α = x + 1
// - β = x
// - γ = x
// - A = x + 3
// - B = x + 2
//
// 经过一次右单旋：
//
//   B     | - α = x + 1
//  / \    | - β = x
// α   A   | - γ = x
//    / \  | - A = x + 1
//   β   γ | - B = x + 2
//
// B 的高度不变但整棵树的高度降低 1。

use crate::{BuddyCollection, BuddyLine, OligarchyCollection};
use core::{fmt, ptr::NonNull};

/// 基于平衡二叉查找树的侵入式伙伴行。
pub struct AvlBuddy {
    tree: Tree,
    base: usize,
    order: usize,
}

impl AvlBuddy {
    #[inline]
    fn ptr_from(&self, idx: usize) -> NonNull<Node> {
        unsafe { NonNull::new_unchecked(((self.base + idx) << self.order) as *mut Node) }
    }
}

impl BuddyLine for AvlBuddy {
    // 每个页上会保存一个 `Node`。
    const MIN_ORDER: usize = core::mem::size_of::<Node>().trailing_zeros() as _;

    const EMPTY: Self = Self {
        tree: Tree(None),
        base: 0,
        order: 0,
    };

    #[inline]
    fn init(&mut self, order: usize, base: usize) {
        self.base = base;
        self.order = order;
    }

    fn take(&mut self, _idx: usize) -> bool {
        todo!()
    }
}

impl OligarchyCollection for AvlBuddy {
    fn take_any(&mut self, _align_order: usize, _count: usize) -> Option<usize> {
        todo!()
    }

    #[inline]
    fn put(&mut self, idx: usize) {
        self.tree.insert(self.ptr_from(idx));
    }
}

impl BuddyCollection for AvlBuddy {
    fn take_any(&mut self, _align_order: usize) -> Option<usize> {
        todo!()
    }

    fn put(&mut self, _idx: usize) -> Option<usize> {
        todo!()
    }
}

impl fmt::Debug for AvlBuddy {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[repr(transparent)]
struct Tree(Option<NonNull<Node>>);

#[repr(C)]
struct Node {
    l: Tree,
    r: Tree,
    h: usize,
}

impl Tree {
    fn insert(&mut self, mut ptr: NonNull<Node>) {
        if let Some(mut root_ptr) = self.0 {
            // 插入结点
            let root = unsafe { root_ptr.as_mut() };
            if ptr < root_ptr {
                &mut root.l
            } else {
                &mut root.r
            }
            .insert(ptr);
            root.update();
            self.rotate();
        } else {
            // 新建结点
            self.0 = Some(ptr);
            *unsafe { ptr.as_mut() } = Node {
                l: Tree(None),
                r: Tree(None),
                h: 1,
            };
        }
    }

    /// 树高。
    ///
    /// 空树高度为 0；单独的结点高度为 1。
    #[inline]
    fn height(&self) -> usize {
        self.0.map_or(0, |node| unsafe { node.as_ref() }.h)
    }

    /// 旋转
    fn rotate(&mut self) {
        let root = unsafe { self.0.unwrap().as_mut() };
        let bf = root.bf();
        if bf > 1 {
            if unsafe { root.l.0.unwrap().as_mut() }.bf() > 0 {
                self.rotate_r();
            } else {
                root.l.rotate_l();
                self.rotate_r();
            }
        } else if bf < -1 {
            if unsafe { root.r.0.unwrap().as_mut() }.bf() < 0 {
                self.rotate_l();
            } else {
                root.r.rotate_r();
                self.rotate_l();
            }
        }
    }

    #[inline]
    /// 右旋
    fn rotate_r(&mut self) {
        use core::mem::replace;
        let a = unsafe { self.0.unwrap().as_mut() };
        let b = unsafe { a.l.0.unwrap().as_mut() };
        //     A    ->    B     |     -->
        //    / \   ->   / \    |    _[A]
        //   B   γ  ->  α   A   |    /|  \
        //  / \     ->     / \  |   /    _\|
        // α   β    ->    β   γ | [B]<----[β]
        self.0 = replace(&mut a.l.0, replace(&mut b.r.0, self.0));
        a.update();
        b.update();
    }

    #[inline]
    /// 左旋
    fn rotate_l(&mut self) {
        use core::mem::replace;
        let a = unsafe { self.0.unwrap().as_mut() };
        let b = unsafe { a.r.0.unwrap().as_mut() };
        //   A      ->      B   |     <--
        //  / \     ->     / \  |     [A]_
        // α   B    ->    A   γ |    /  |\
        //    / \   ->   / \    |  |/_    \
        //   β   γ  ->  α   β   | [β]---->[B]
        self.0 = replace(&mut a.r.0, replace(&mut b.l.0, self.0));
        a.update();
        b.update();
    }
}

impl Node {
    /// 更新结点。
    #[inline]
    fn update(&mut self) {
        // 结点高度比左右子树中高的高 1
        self.h = core::cmp::max(self.l.height(), self.r.height()) + 1;
    }

    /// 平衡因子。
    #[inline]
    fn bf(&self) -> isize {
        self.l.height() as isize - self.r.height() as isize
    }
}
