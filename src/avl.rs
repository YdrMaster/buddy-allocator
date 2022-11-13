//! 没有实现线程安全
//! 运算逻辑：
//! 在进行存储的时候存在如下空间
//!
//! 空间序号：1,2,3,4,5,6,7,8
//! 是否分配：0,1,1,1,0,0,0,0
//!
//! 此时对应的层级以及逐步删除划分为：
//! 1:1 |1:x |1:    |1:5,6|1:x,6|1:x|1:7,8|1:x,8 |1:x
//! 2:  |2:  |2:5,7 |2:x,7|2:7  |2:7|2:x  |2:    |2:
//! 3:5 |3:5 |3:x   |3:   |3:   |3: |3:   |3:    |3:
//!
//! 因此，在进行插入的时候，如果发现存在两个类似的idx，比如7,8，则代表可能出现重复的情况
//! 此时的想法在于添加一个新的函数，其主要作用在于删除一个制定ptr的节点（已经确定存在）通过这种方式来实现对于兄弟节点的回收
//! 同时，在完成回收之后，将两个兄弟节点中的最小的节点返回给上一级（并且插入到上一级的avl树中）
//!
//! 同时，从总体上来说：在某一层删除一个节点其实相当于从avl树中获取一个节点（按照我目前的算法来说，只有删除的idx是根节点才会出错）
//! 在如果在某一层没有找到对应元素，则向其上一层进行借用，在借用完成的时候再将其中的一部分插入到下一层的位置（这个地方按照lib是可以递归的）
//! **层级代表着size**
//!
//! 目前更倾向与将代表着上一层的元素保留，而先分配另外一个

// AVL 算法
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

use crate::{BuddyCollection, BuddyLine, OligarchyCollection, Order};
use core::{fmt, ptr::NonNull};
/// 基于平衡二叉查找树的侵入式伙伴行。
pub struct AvlBuddy {
    tree: Tree,
    base: usize,
    order: Order,
}

// #[allow(dead_code)]
// impl AvlBuddy {
//     #[inline]
//     fn ptr_from(&self, idx: usize) -> NonNull<Node> {
//         unsafe { NonNull::new_unchecked(((self.base + idx) << self.order) as *mut Node) }
//     }
// }

impl BuddyLine for AvlBuddy {
    // 每个页上会保存一个 `Node`。
    const MIN_ORDER: usize = core::mem::size_of::<Node>().trailing_zeros() as _;

    const EMPTY: Self = Self {
        tree: Tree(None),
        base: 0,
        order: Order::new(0),
    };

    #[inline]
    fn init(&mut self, order: usize, base: usize) {
        self.base = base;
        self.order = Order::new(order);
    }

    fn take(&mut self, _idx: usize) -> bool {
        todo!()
    }
}

impl OligarchyCollection for AvlBuddy {
    fn take_any(&mut self, _align_order: usize, _count: usize) -> Option<usize> {
        // 个人感觉基于寡头行的数量而言，实现这个地方没有什么效率
        todo!()
    }

    #[inline]
    fn put(&mut self, _idx: usize) {
        todo!()
    }
}

impl BuddyCollection for AvlBuddy {
    // get a node from avl_buddy
    fn take_any(&mut self, _align_order: usize) -> Option<usize> {
        // 默认以相同大小进行分配我感觉是比较好的，但是不排除后面修改了想法
        // TODO 需要考虑是否进行边界判断
        if _align_order != 0 {
            None
        } else {
            self.tree.delete(&self.order)
        }
    }

    /// insert node into avl_buddy
    fn put(&mut self, idx: usize) -> Option<usize> {
        // 需要额外考虑一个事情，就是在进行分配的时候，最小分配单元必须大于Node，因为这个Node实际上是存放在分配的空间中的，因此需要加入判定以确保空间不会出现重叠的情况
        if self.order.0 >= <Self as BuddyLine>::MIN_ORDER && self.tree.insert(idx, &self.order) {
            None
        } else {
            // find it's buddy
            /* DEBUG */
            // println!("facing it's buddy");
            // Some(idx & (!(1)))
            Some(idx >> 1)
        }
    }
}

impl fmt::Debug for AvlBuddy {
    /// 以序列化前序遍历的方式输出
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // todo!("这个地方需要考虑到对于二叉搜索树的便利情况，可能比较难完成")
        // 这个地方我认为相对来说较难复现前面的算法,即使用层序便利对于结果进行呈现,可能需要采用标准搜索方案来对于结果进行呈现
        // 同时由于在trait外部实现dfs算法相对比较困难(感觉会造成割裂感),因此采用内部函数递归来实现这个操作
        write!(f, "[")?;

        fn dfs(root: &Tree, order: &Order, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // if it's leaf node
            match root.0 {
                // if it's leaf node
                None => write!(f, "#,"),
                Some(root_node) => {
                    // let a = root_node.as_ref().l;
                    write!(f, "{:#x}[{:?}],", order.ptr_to_idx(root_node), unsafe {
                        root_node.as_ref().h
                    })?;
                    // write!(f, "{:#x}[{:?}],", root_node.as_ptr() as usize, unsafe { root_node.as_ref().h })?;
                    // write!(f, "{:#x},", root_node.as_ptr() as usize)?;
                    dfs(unsafe { &root_node.as_ref().l }, order, f)?;
                    // write!(f, "{:#x},", root_node.as_ptr() as usize)?;
                    dfs(unsafe { &root_node.as_ref().r }, order, f)
                }
            }
            // if let None = root.0 {
            //     write!(f, "#")
            // }
            // else {
            //     write!(f, "{:X}", root.0.)
            // }
        }

        dfs(&self.tree, &self.order, f)?;
        write!(f, "]")
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.l.0.is_some() {
            write!(f, "l:{:#x?} ", self.l.0.unwrap().as_ptr() as usize)?;
        }
        else {
            write!(f, "l:null ")?;
        }
        if self.r.0.is_some() {
            write!(f, "r:{:#x?} ", self.r.0.unwrap().as_ptr() as usize)?;
        }
        else {
            write!(f, "r:null ")?;
        }
        write!(f, "h:{}", self.h)
    }
}

impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_some() {
            write!(f, "{:?}", self.0.unwrap())
        }
        else {
            write!(f, "null")
        }
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

/// 以递归方式找到距离传入子树最大节点最近的子树节点
/// 注意：调用者需要考虑到传出节点可能存在反向子树的情况
fn find_max(node: &NonNull<Node>) -> NonNull<Node> {
    // 需要考虑到有左子树的情况
    //  [A]    
    //    \
    //     [B] (node.r)
    //    /
    //   [?]
    if unsafe { node.as_ref().r.0.is_some() && node.as_ref().r.0.unwrap().as_ref().r.0.is_some() } {
        // if have right and it's right
        find_max(unsafe { &node.as_ref().r.0.unwrap() })
    } else {
        node.clone()
    }
}

/// 以递归方式找到并返回距离最小子树最近的子树
/// 注意：调用者需要考虑到可能传出节点存在反向子树的情况
fn find_min(node: &NonNull<Node>) -> NonNull<Node> {
    // 需要考虑有右子树的情况
    //    [a]          
    //    /
    //   [b]            (min point)
    //    \
    //     [c]
    if unsafe { node.as_ref().l.0.is_some() && node.as_ref().l.0.unwrap().as_ref().l.0.is_some() } {
        find_min(unsafe { &node.as_ref().l.0.unwrap() })
    } else {
        node.clone()
    }
}

impl Tree {
    /// 找到并返回距离最大子树最近的子树
    fn insert(&mut self, idx: usize, order: &Order) -> bool {
        // 这个地方我认为目前的速度瓶颈主要出现在大量使用递归所带来的影响，但是考虑到本lab主要完成的是分配器，因此可能没有办法实现动态的内存分配，进而使用栈或者队列来实现对应操作

        // 版本二：在进行插入的时候直接完成对应伙伴节点的删除工作
        let ptr: NonNull<Node> = unsafe { order.idx_to_ptr(idx) };
        match self.0 {
            // if this node is not empty
            Some(mut root_ptr) => {
                let root = unsafe { root_ptr.as_mut() };
                let buddy = unsafe { order.idx_to_ptr(idx ^ 1) };
                use core::cmp::Ordering::*;
                // use core::mem::replace;

                // 找到我们需要去的方向
                let ret = match root_ptr.cmp(&buddy) {
                    Less => {
                        // 向右前方前进， 并且右边子树节点为伙伴节点 => 此时做可能出现的所有删除情形的判断
                        if root.r.0.is_some() && order.ptr_to_idx(root.r.0.unwrap()) == idx ^ 1 {
                            // 要删除的节点
                            let node = unsafe { root.r.0.unwrap().as_mut() };
                            // 测试两个子树的状态
                            match (node.l.0.is_some(), node.r.0.is_some()) {
                                (true, true) => {
                                    // have both left and right subtree
                                    if node.l.height() < node.r.height() {
                                        // right tree higher than left tree
                                        // root:    the point you using the function
                                        // node:    the point you want to delete
                                        // byond:   the point byond the leaf
                                        //  [A]   (root)    |   [A]
                                        //  / \             |   / \
                                        // .. [B] (node)    |  ..  [E]
                                        //    /       \     |      /  \
                                        //  [C](byond)[D]   |    [C]  [D]
                                        //    \             |     \
                                        //    [E] (leaf)    |     [F]
                                        //     \
                                        //      [F]
                                        if node.r.height() == 1 {
                                            // right subtree is leaf => delete node
                                            let leaf = unsafe { node.r.0.unwrap().as_mut() };
                                            leaf.l = Tree(node.l.0);
                                            root.r = Tree(NonNull::new(leaf));
                                            leaf.update();
                                            root.update();
                                        } else {
                                            // right subtree has subtree => delete link and relink this link to it's parent then delete node
                                            let mut beyond = unsafe { find_min(&node.l.0.unwrap()).as_mut() };

                                            if let Some(mut leaf) = beyond.l.0 {
                                                // 如果 beyond 存在左节点 
                                                let leaf = unsafe { leaf.as_mut() };

                                                // 如果 leaf 存在反方向节点
                                                if leaf.r.0.is_some() {
                                                    beyond.l = Tree(leaf.l.0);
                                                    leaf.l = Tree(None);
                                                } else {
                                                    beyond.l = Tree(None);
                                                }
                                                leaf.l = Tree(node.l.0);
                                                leaf.r = Tree(node.r.0);
                                                root.r = Tree(NonNull::new(leaf));
                                                leaf.update();
                                                root.update();    
                                            } else {
                                                // 将 beyond 作为 leaf 替换 node 的位置
                                                beyond.l = Tree(node.l.0);
                                                root.r = Tree(NonNull::new(beyond));
                                                beyond.update();
                                                root.update();
                                            }
                                        }
                                    } else {
                                        // left tree higher than right subtree
                                        // root:    the point you using the function
                                        // node:    the point you want to delete
                                        // byond:   the point byond the leaf
                                        // [A]   (root)     | [A]
                                        //   \              |  \
                                        //   [B] (node)     |   [E]
                                        //   / \            |   / \
                                        //  [C] [D](byond)  | [C] [D]
                                        //      /           |       \
                                        // (E:leaf)         |        [F]
                                        //     \
                                        //     [F]
                                        if node.l.height() == 1 {
                                            // left subtree is leaf => delete node
                                            let leaf = unsafe { node.l.0.unwrap().as_mut() };
                                            leaf.r = Tree(node.r.0);
                                            root.r = Tree(NonNull::new(leaf));
                                            leaf.update();
                                            root.update();
                                        } else {
                                            #[allow(unused_variables, unused_mut, dead_code)]
                                            // left subtree has subtree => delete link and relink this link to it's parent then delete node
                                            let mut beyond = unsafe { find_max(&node.l.0.unwrap()).as_mut() };

                                            if let Some(mut leaf) = beyond.r.0 {
                                                // 如果 beyond 存在右子树【这个实际上是为了弥补 find_max 中无法获取这种情况下距离最大节点最近的节点设计的】
                                                let leaf = unsafe { leaf.as_mut() };

                                                // 如果 leaf 存在反方向子树
                                                if leaf.l.0.is_some() {
                                                    beyond.r = Tree(leaf.l.0);
                                                    leaf.l = Tree(None);
                                                }
                                                else {
                                                    beyond.r = Tree(None);
                                                }
                                                leaf.l = Tree(node.l.0);
                                                leaf.r = Tree(node.r.0);
                                                root.r = Tree(NonNull::new(leaf));
                                                leaf.update();
                                                root.update();
                                            } else {
                                                // 将 beyond 作为 leaf 替换 node 的位置
                                                beyond.r = Tree(node.r.0);
                                                root.r = Tree(NonNull::new(beyond));   
                                                beyond.update();
                                                root.update();
                                            }
                                        }
                                    }
                                    node.update();
                                    root.r.rotate();
                                }
                                (true, false) => {
                                    // have left but not right
                                    // [A] <- root   | [A]
                                    //   \           |   \
                                    //    [B] <- node|  [C]
                                    //    /          |
                                    //  [C]          |
                                    root.r = Tree(Some(node.l.0.unwrap()));
                                    node.l = Tree(None); // 可有可无？
                                    root.update();
                                }
                                (false, true) => {
                                    // have left but not right
                                    // [A] <- root   | [A]
                                    //   \           |   \
                                    //    [B] <- node|  [C]
                                    //      \        |
                                    //      [C]      | 这个地方不能直接上旋转，将节点转到叶子的原因是太麻烦了
                                    root.r = Tree(Some(node.r.0.unwrap()));
                                    node.r = Tree(None);
                                    root.update();
                                }
                                (false, false) => {
                                    root.r = Tree(None);
                                    root.update();
                                }
                            }
                            return false;
                        } else {
                            // 如果前进的方向不是 buddy 节点，则以递归方式继续进行插入，一直到达对应的位置(插入于空节点)
                            root.r.insert(idx, order)
                        }
                    }
                    Equal => {
                        // 个人感觉这个地方只可能出现在根节点处，因此一旦出现，则置当前节点为空
                        // self.0 = None;
                        // return true
                        match (root.l.0.is_some(), root.r.0.is_some()) {
                            (true, true) => {
                                match (
                                    root.l.height() < root.r.height(),
                                    core::cmp::max(root.l.height(), root.r.height()),
                                ) {
                                    (_, 1) => {
                                        // if both of left and right is leaf => choice left as root
                                        let left = unsafe { root.l.0.unwrap().as_mut() };
                                        // left.l = Tree(None);
                                        left.r = Tree(root.r.0);
                                        self.0 = NonNull::new(left);
                                        left.update();
                                    }
                                    (true, _) => {
                                        // right higher that left
                                        // let mut beyond =
                                        //     unsafe { find_min(&root.r.0.unwrap()).as_mut() };
                                        // let mut leaf = unsafe { beyond.l.0.unwrap().as_mut() };
                                        // if leaf.r.0.is_some() {
                                        //     beyond.l = Tree(leaf.r.0);
                                        //     leaf.r = Tree(None);
                                        // } else {
                                        //     beyond.l = Tree(None);
                                        // }
                                        // leaf.l = Tree(root.l.0);
                                        // leaf.r = Tree(root.r.0); // BC 高度一定大于1,因此不会出现循环的情况
                                        // self.0 = NonNull::new(leaf);
                                        // leaf.update();

                                        let mut beyond = unsafe { find_min(&root.r.0.unwrap()).as_mut() };
                                        
                                        if let Some(mut leaf) = beyond.l.0 {
                                            let leaf = unsafe { leaf.as_mut() };

                                            if leaf.r.0.is_some() {
                                                beyond.l = Tree(leaf.r.0);
                                                leaf.r = Tree(None);
                                            } else {
                                                beyond.l = Tree(None);
                                            }
                                            leaf.l = Tree(root.l.0);
                                            leaf.r = Tree(root.r.0);
                                            self.0 = NonNull::new(leaf);
                                            leaf.update();
                                        }
                                        else {
                                            // 取右边最高节点出来作为root
                                            let mut right = unsafe { root.r.0.unwrap().as_mut() };
                                            right.l = Tree(root.l.0);
                                            self.0 = NonNull::new(right);
                                            right.update();
                                        }
                                    }
                                    (false, _) => {
                                        // left higher than right
                                        // let mut beyond =
                                        //     unsafe { find_max(&root.l.0.unwrap()).as_mut() };
                                        // let mut leaf = unsafe { beyond.r.0.unwrap().as_mut() };
                                        // if leaf.l.0.is_some() {
                                        //     beyond.r = Tree(leaf.l.0);
                                        //     leaf.l = Tree(None);
                                        // } else {
                                        //     beyond.r = Tree(None);
                                        // }
                                        // leaf.l = Tree(root.l.0);
                                        // leaf.r = Tree(root.r.0);
                                        // self.0 = NonNull::new(leaf);
                                        // leaf.update();
                                        let mut beyond = unsafe { find_max(&root.l.0.unwrap()).as_mut() };
                                        if let Some(mut leaf) = beyond.r.0 {
                                            let leaf = unsafe { leaf.as_mut() };

                                            if leaf.l.0.is_some() {
                                                beyond.r = Tree(leaf.l.0);
                                                leaf.l = Tree(None);
                                            } else {
                                                beyond.r = Tree(None);
                                            }
                                            leaf.l = Tree(root.l.0);
                                            leaf.r = Tree(root.r.0);
                                            self.0 = NonNull::new(leaf);
                                            leaf.update();
                                        } else {
                                            let mut left = unsafe { root.l.0.unwrap().as_mut() };
                                            left.r = Tree(root.r.0);
                                            self.0 = NonNull::new(left);
                                            left.update()
                                        }

                                    }
                                }
                            }
                            (true, false) => {
                                self.0 = Some(root.l.0.unwrap());
                            }
                            (false, true) => {
                                self.0 = Some(root.r.0.unwrap());
                            }
                            (false, false) => {
                                self.0 = None;
                                return false;
                            }
                        };

                        false
                    }
                    Greater => {
                        // 向左方前进，前进前确认对应节点是否存在，以及节点是否是buddy
                        if root.l.0.is_some() && order.ptr_to_idx(root.l.0.unwrap()) == idx ^ 1 {
                            // if delete node is not leaf => delete link
                            let node = unsafe { root.l.0.unwrap().as_mut() };
                            match (node.l.0.is_some(), node.r.0.is_some()) {
                                (true, true) => {
                                    // have both left and right subtree
                                    if node.l.height() < node.r.height() {
                                        // right tree higher than left tree
                                        // node:    the point you want to delete
                                        // byond:   the point byond the leaf
                                        //      [A] (root)  |      [A]
                                        //     /            |       /
                                        // . [B] (node)     |      [E]
                                        //    /       \     |      /  \
                                        //  [C](byond)[D]   |    [C]  [D]
                                        //    \             |     \
                                        //    [E] (leaf)    |     [F]
                                        //     \
                                        //      [F]// 找到距离左子树最大节点最近的节点
                                        if node.r.height() == 1 {
                                            let leaf = unsafe { node.r.0.unwrap().as_mut() };
                                            leaf.l = Tree(node.l.0);
                                            root.l = Tree(NonNull::new(leaf));
                                            leaf.update();
                                            root.update();
                                        } else {
                                            let mut beyond = unsafe { find_min(&node.r.0.unwrap()).as_mut() };
                                            
                                            if let Some(mut leaf) = beyond.l.0 {
                                                // 如果 beyond 存在左子树
                                                let mut leaf = unsafe { leaf.as_mut() };
                                                
                                                // 如果存在反方向子树
                                                if leaf.r.0.is_some() {
                                                    beyond.l = Tree(leaf.r.0);
                                                    leaf.r = Tree(None);
                                                } else {
                                                    beyond.l = Tree(None);
                                                }

                                                leaf.l = Tree(node.l.0);
                                                leaf.r = Tree(node.r.0);
                                                root.l = Tree(NonNull::new(leaf));

                                                leaf.update();
                                                root.update();
                                            }
                                            else {
                                                // 将 beyond 作为 leaf 替换 node
                                                beyond.l = Tree(node.l.0);
                                                root.l = Tree(NonNull::new(beyond));
                                                beyond.update();
                                                root.update();
                                            }
                                        }
                                    } else {
                                        // left tree higher than right subtree
                                        // node:    the point you want to delete
                                        // byond:   the point byond the leaf
                                        //    [A]   (root)  |     [A]
                                        //    /             |    /
                                        //   [B] (node)     |   [E]
                                        //   / \            |   / \
                                        //  [C] [D](byond)  | [C] [D]
                                        //      /           |       \
                                        // (E:leaf)         |        [F]
                                        //     \
                                        //     [F]
                                        if node.l.height() == 1 {
                                            // left subtree is leaf => delete node
                                            let leaf = unsafe { node.l.0.unwrap().as_mut() };
                                            leaf.r = Tree(node.r.0);
                                            root.l = Tree(NonNull::new(leaf));
                                            leaf.update();
                                            root.update();
                                        } else {
                                            // left subtree has subtree => delete link and relink this link to it's parent then delete node
                                            // let mut beyond = unsafe { find_min(&node.l.0.unwrap()).as_mut() };
                                            // dbg!("{:?}", &beyond);
                                            // let leaf = unsafe { beyond.r.0.unwrap().as_mut() };
                                            // if leaf.l.0.is_some() {
                                            //     beyond.r = Tree(leaf.l.0);
                                            //     leaf.l = Tree(None);
                                            // } else {
                                            //     beyond.r = Tree(None);
                                            // }
                                            // leaf.l = Tree(node.l.0);
                                            // leaf.r = Tree(node.r.0);
                                            // root.l = Tree(NonNull::new(leaf));
                                            // leaf.update();
                                            // root.update();
                                            let mut beyond = unsafe { find_max(&node.l.0.unwrap()).as_mut() };
                                            if let Some(mut leaf) = beyond.r.0 {
                                                let leaf = unsafe { leaf.as_mut() };

                                                if leaf.l.0.is_some() {
                                                    beyond.r = Tree(leaf.l.0);
                                                    leaf.l = Tree(None);
                                                } else {
                                                    beyond.r = Tree(None);
                                                }
                                                leaf.l = Tree(node.l.0);
                                                leaf.r = Tree(node.r.0);
                                                root.l = Tree(NonNull::new(leaf));
                                                leaf.update();
                                                root.update();
                                            } else {
                                                beyond.r = Tree(node.r.0);
                                                root.l = Tree(NonNull::new(beyond));
                                                beyond.update();
                                                root.update();
                                            }
                                        }
                                    }
                                }
                                (true, false) => {
                                    // have left but not right
                                    //      [A] <- root |   [A]
                                    //     /            |   /
                                    //    [B] <- node   |  [C]
                                    //    /             |
                                    //  [C]             |
                                    root.l = Tree(Some(node.l.0.unwrap()));
                                    node.l = Tree(None);
                                    node.update();
                                }
                                (false, true) => {
                                    // have left but not right
                                    //      [A] <- root |    [A]
                                    //      /           |   /
                                    //    [B] <- node   |  [C]
                                    //      \           |
                                    //      [C]         | 这个地方不能直接上旋转，将节点转到叶子的原因是太麻烦了   // have right node but not left
                                    root.l = Tree(Some(node.r.0.unwrap()));
                                    node.l = Tree(None);
                                    node.update();
                                }
                                (false, false) => {
                                    // is leaf node
                                    root.l = Tree(None);
                                    root.update();
                                }
                            }
                            return false;
                        } else {
                            // 如果前进的方向不是 buddy 节点，则以递归方式继续进行插入，一直到达对应的位置(插入于空节点)
                            root.l.insert(idx, order)
                        }
                    }
                };
                root.update();
                self.rotate();
                ret
            }
            // if this node is empty => insert in this point
            None => {
                self.0 = Some(ptr);
                *unsafe { order.idx_to_ptr(idx).as_mut() } = Node {
                    l: Tree(None),
                    r: Tree(None),
                    h: 1,
                };
                true
            }
        }
    }

    /// 从地址池中获取一个单位的地址, 并且返回这个地址
    fn delete(&mut self, order: &Order) -> Option<usize> {
        /*
        根据需求，此处需要实现的子模块包括，通过 左右子树中的最小高度导航到 到最近的叶子结点，然后再进行 删除节点操作
        删除节点操作的时候，由于当前操作在叶子节点处产生，因此不需要考虑额外信息，直接将其删除即可
        */
        match self.0 {
            None => None,
            Some(mut root_ptr) => {
                let root = unsafe { root_ptr.as_mut() };
                let ret = match (root.l.0.is_some(), root.r.0.is_some()) {
                    (true, true) => {
                        // have both left and right subtree
                        // 如果某个方向只剩下一个节点，则直接删除这个节点，并且返回对应信息
                        // 否则以递归方式继续进行
                        // 这个地方实际上主要目的在于减少代码量...但是反而带来了可读性的降低
                        match (
                            root.l.height() < root.r.height(),
                            core::cmp::min(root.l.height(), root.r.height()),
                        ) {
                            (true, 1) => {
                                let node = order.ptr_to_idx(root.l.0.unwrap());
                                root.l = Tree(None);
                                root.update();
                                return Some(node);
                            }
                            (true, _) => root.l.delete(order),
                            (false, 1) => {
                                let node = order.ptr_to_idx(root.r.0.unwrap());
                                root.r = Tree(None);
                                root.update();
                                return Some(node);
                            }
                            (false, _) => root.r.delete(order),
                        }
                    }
                    (true, false) => {
                        // only have left subtree
                        match root.l.height() {
                            1 => {
                                let node = order.ptr_to_idx(root.l.0.unwrap());
                                root.l = Tree(None);
                                root.update();
                                return Some(node);
                            }
                            _ => root.l.delete(order),
                        }
                    }
                    (false, true) => {
                        // only have right subtree
                        match root.r.height() {
                            1 => {
                                let node = order.ptr_to_idx(root.r.0.unwrap());
                                root.r = Tree(None);
                                root.update();
                                return Some(node);
                            }
                            _ => root.r.delete(order),
                        }
                    }
                    (false, false) => {
                        // this is the root node (and it's leaf) => clean itself
                        // let node = self.0.unwrap().as_ptr() as usize;
                        let node = order.ptr_to_idx(self.0.unwrap());
                        self.0 = None;
                        return Some(node);
                    }
                };
                root.update();
                self.rotate();
                ret
            }
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

#[cfg(test)]
#[allow(unused_variables, dead_code)]
mod test {
    // 这个地方主要是基于白盒测试点需要实现的，但是随之出现的问题在于其依赖项(pirnt（删除不方便测试），vec[非——必要]）
    // 没有办法在no_std的情况下运行，也因 rust pub use only for tests 相对比较复杂，同时可能有隔离的问题，因而没有使用这个方法
    // 也不太能转换成为 write 宏进行实现
    use super::*;

    #[repr(C, align(4096))]
    struct Page([u8; 4096]);

    impl Page {
        const ZERO: Self = Self([0; 4096]);
    }

    impl Node {
        const EMPTY: Node = Self {
            l: Tree(None),
            r: Tree(None),
            h: 1,
        };
    }

    /// 256 MiB
    static mut MEMORY: [Page; 1024] = [Page::ZERO; 1024];
    // 彼此之间要间隔开至少24个数字以防止某种程度上的冲突
    // NonNull<Node>: 8; Node: 24; u8:1
    // 0  64  128  192  256  320  384  448  512  576  640  704  768  832  896  960
    fn create_nonnull_list() -> Vec<NonNull<Node>> {
        let mut list = Vec::new();
        for i in 0..1024 / 64 {
            list.push(unsafe {
                NonNull::new_unchecked(MEMORY[i * 64].0.as_mut_ptr() as *mut Node)
            });
        }
        println!("num\tidx\t\tptr");
        for i in 0..list.len() {
            print!("> {i}:\t");
            println!("{:#x?}\t", (list[i].as_ptr() as usize) >> ORDER_LEVEL);
            // println!("{:#x?}", (list[i].as_ptr() as usize));
        }
        list
    }

    use crate::{AvlBuddy, BuddyCollection};
    const ORDER_LEVEL: usize = 12;
    /* TEST FOR BASAL INSERT OPERATION */
    #[test]
    fn test_for_insert_l() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[7].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[9].as_ptr() as usize) >> ORDER_LEVEL);
        // <avlbuddy as buddycollection>::put(&mut avl_buddy, (vec[6].as_ptr() as usize) >> order_level);

        // let a = unsafe { &avl_buddy.tree.0.unwrap().as_ref().l};
        println!("{avl_buddy:#x?}");
        assert_eq!(avl_buddy.tree.0.unwrap(), vec[8]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().l.0.unwrap() }, vec[7]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().r.0.unwrap() }, vec[9]);
    }
    #[test]
    fn test_for_insert_r() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[9].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[7].as_ptr() as usize) >> ORDER_LEVEL);
        // <avlbuddy as buddycollection>::put(&mut avl_buddy, (vec[6].as_ptr() as usize) >> order_level);

        // let a = unsafe { &avl_buddy.tree.0.unwrap().as_ref().l};
        println!("{avl_buddy:#x?}");
        assert_eq!(avl_buddy.tree.0.unwrap(), vec[8]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().l.0.unwrap() }, vec[7]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().r.0.unwrap() }, vec[9]);
    }
    #[test]
    fn test_for_insert_lr() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[9].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[7].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);
        // <avlbuddy as buddycollection>::put(&mut avl_buddy, (vec[6].as_ptr() as usize) >> order_level);

        // let a = unsafe { &avl_buddy.tree.0.unwrap().as_ref().l};
        println!("{avl_buddy:#x?}");
        assert_eq!(avl_buddy.tree.0.unwrap(), vec[8]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().l.0.unwrap() }, vec[7]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().r.0.unwrap() }, vec[9]);
    }
    #[test]
    fn test_for_insert_rl() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[7].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[9].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);

        println!("{avl_buddy:#x?}");
        assert_eq!(avl_buddy.tree.0.unwrap(), vec[8]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().l.0.unwrap() }, vec[7]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().r.0.unwrap() }, vec[9]);
    }
    /* TEST FOR BUDDY OPERATION: DELETE BUDDY NODE AND NOT INSERT */
    #[test]
    fn test_for_insert_buddy_root() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        let root_idx = (vec[7].as_ptr() as usize) >> ORDER_LEVEL;
        let buddy_idx = ((vec[7].as_ptr() as usize) >> ORDER_LEVEL) ^ 1;
        println!("{root_idx:#x?}\t{buddy_idx:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, root_idx);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, buddy_idx);

        println!("{avl_buddy:#x?}");
        assert_eq!(avl_buddy.tree.0, None);
    }
    #[test]
    fn test_for_insert_buddy_left_leaf() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        let buddy_idx = ((vec[7].as_ptr() as usize) >> ORDER_LEVEL) ^ 1;
        println!("{buddy_idx:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[7].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[9].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, buddy_idx);

        println!("{avl_buddy:#x?}");
        assert_eq!(avl_buddy.tree.0.unwrap(), vec[8]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().l.0 }, None);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().r.0.unwrap() }, vec[9]);
    }
    #[test]
    fn test_for_insert_buddy_right_leaf() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        let buddy_idx = ((vec[9].as_ptr() as usize) >> ORDER_LEVEL) ^ 1;
        println!("{buddy_idx:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[7].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[9].as_ptr() as usize) >> ORDER_LEVEL);
        println!("{avl_buddy:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, buddy_idx);
        println!("{avl_buddy:#x?}");
        assert_eq!(avl_buddy.tree.0.unwrap(), vec[8]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().l.0.unwrap() }, vec[7]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().r.0 }, None);
    }
    #[test]
    fn test_for_insert_buddy_right_not_leaf_right_only() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        let buddy_idx = ((vec[9].as_ptr() as usize) >> ORDER_LEVEL) ^ 1;
        println!("{buddy_idx:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[6].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[9].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[4].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[10].as_ptr() as usize) >> ORDER_LEVEL);
        println!("{avl_buddy:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, buddy_idx);
        println!("{avl_buddy:#x?}");
        assert_eq!(avl_buddy.tree.0.unwrap(), vec[8]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().r.0.unwrap() }, vec[10]);
    }
    #[test]
    fn test_for_insert_buddy_right_not_leaf_left_only() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        let buddy_idx = ((vec[10].as_ptr() as usize) >> ORDER_LEVEL) ^ 1;
        println!("{buddy_idx:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[6].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[10].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[4].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[9].as_ptr() as usize) >> ORDER_LEVEL);
        println!("{avl_buddy:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, buddy_idx);

        println!("{avl_buddy:#x?}");
        assert_eq!(avl_buddy.tree.0.unwrap(), vec[8]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().r.0.unwrap() }, vec[9]);
    }
    #[test]
    fn test_for_insert_buddy_right_not_leaf_both_no_subleaf() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        let buddy_idx = ((vec[10].as_ptr() as usize) >> ORDER_LEVEL) ^ 1;
        println!("{buddy_idx:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[6].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[10].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[4].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[9].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[11].as_ptr() as usize) >> ORDER_LEVEL);
        println!("{avl_buddy:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, buddy_idx);
        println!("{avl_buddy:#x?}");
        assert_eq!(avl_buddy.tree.0.unwrap(), vec[8]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().r.0.unwrap() }, vec[11]);
    }
    #[test]
    fn test_for_insert_buddy_right_not_leaf_both_with_subleaf() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        let buddy_idx = ((vec[10].as_ptr() as usize) >> ORDER_LEVEL) ^ 1;
        println!("{buddy_idx:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[6].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[10].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[4].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[9].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[12].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[2].as_ptr() as usize) >> ORDER_LEVEL);
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[11].as_ptr() as usize) >> ORDER_LEVEL);
        println!("{avl_buddy:#x?}");
        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, buddy_idx);
        println!("{avl_buddy:#x?}");
        assert_eq!(avl_buddy.tree.0.unwrap(), vec[8]);
        assert_eq!( unsafe{ avl_buddy.tree.0.unwrap().as_ref().r.0.unwrap() }, vec[11]);
    }
    #[test]
    fn test_for_delete_root() {
        let vec = create_nonnull_list();
        println!("{}", vec.len());
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);
        let a = <AvlBuddy as BuddyCollection>::take_any(&mut avl_buddy, 0);

        assert_eq!(avl_buddy.tree.0, None);
    }

    // This test is not for testing, but checking tree action by manaual(due to different implement in selection)
    #[test]
    fn test_for_insert_and_delete() {
        let vec = create_nonnull_list();
        let mut avl_buddy = AvlBuddy::EMPTY;
        avl_buddy.init(ORDER_LEVEL, vec[0].as_ptr() as usize);

        <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[6].as_ptr() as usize) >> ORDER_LEVEL);
        let a = unsafe { avl_buddy.tree.0.unwrap().as_ref() };
        println!("{:?}", a);
        // <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[4].as_ptr() as usize) >> ORDER_LEVEL);
        // <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[5].as_ptr() as usize) >> ORDER_LEVEL);
        // <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[8].as_ptr() as usize) >> ORDER_LEVEL);
        // <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[9].as_ptr() as usize) >> ORDER_LEVEL);
        // <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[7].as_ptr() as usize) >> ORDER_LEVEL);
        // <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[10].as_ptr() as usize) >> ORDER_LEVEL);
        // <AvlBuddy as BuddyCollection>::put(&mut avl_buddy, (vec[11].as_ptr() as usize) >> ORDER_LEVEL);

        // println!("{avl_buddy:#x?}");
        // assert_eq!(vec[4].as_ptr() as usize >> ORDER_LEVEL, <AvlBuddy as BuddyCollection>::take_any(&mut avl_buddy, 0).unwrap());
        // println!("{avl_buddy:#x?}");
        // assert_eq!(vec[7].as_ptr() as usize >> ORDER_LEVEL, <AvlBuddy as BuddyCollection>::take_any(&mut avl_buddy, 0).unwrap());
        // println!("{avl_buddy:#x?}");
        // assert_eq!(vec[5].as_ptr() as usize >> ORDER_LEVEL, <AvlBuddy as BuddyCollection>::take_any(&mut avl_buddy, 0).unwrap());
        // println!("{avl_buddy:#x?}");
        // assert_eq!(vec[6].as_ptr() as usize >> ORDER_LEVEL, <AvlBuddy as BuddyCollection>::take_any(&mut avl_buddy, 0).unwrap());
        // println!("{avl_buddy:#x?}");
        // assert_eq!(vec[11].as_ptr() as usize >> ORDER_LEVEL, <AvlBuddy as BuddyCollection>::take_any(&mut avl_buddy, 0).unwrap());
        // println!("{avl_buddy:#x?}");
        // assert_eq!(vec[10].as_ptr() as usize >> ORDER_LEVEL, <AvlBuddy as BuddyCollection>::take_any(&mut avl_buddy, 0).unwrap());
        // println!("{avl_buddy:#x?}");
        // assert_eq!(vec[8].as_ptr() as usize >> ORDER_LEVEL, <AvlBuddy as BuddyCollection>::take_any(&mut avl_buddy, 0).unwrap());
        // println!("{avl_buddy:#x?}");
        // assert_eq!(vec[9].as_ptr() as usize >> ORDER_LEVEL, <AvlBuddy as BuddyCollection>::take_any(&mut avl_buddy, 0).unwrap());
        // assert_eq!(None, <AvlBuddy as BuddyCollection>::take_any(&mut avl_buddy, 0));
        // println!("{avl_buddy:#x?}");
    }
}
