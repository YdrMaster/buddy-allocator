﻿// # 静止的 AVL 树左右子树的高度差的绝对值最大为 1
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

#[allow(dead_code)]
/// 基于平衡二叉查找树的侵入式伙伴行。
pub struct AvlBuddy {
    tree: Tree,
    base: usize,
    order: usize,
}

#[allow(dead_code)]
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
    fn take_any(&mut self, align_order: usize, count: usize) -> Option<usize> {
        // 个人感觉基于寡头行的数量而言，实现这个地方没有什么效率
        if count > 1 || align_order > 0 {
            None
        }
        else {
            self.tree.delete()
        }
    }

    #[inline]
    fn put(&mut self, idx: usize) {
        self.tree.insert(self.ptr_from(idx));
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
            self.tree.delete()
        }
    }

    /// insert node into avl_buddy
    fn put(&mut self, _idx: usize) -> Option<usize> {
        let node = self.ptr_from(_idx);
        if self.tree.insert(node) {
            // correct insert a node into avl_buddy
            None
        }        
        else {
            // 暂且不明白
            Some(_idx>>1)
        }
    }
}

impl fmt::Debug for AvlBuddy {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("这个地方需要考虑到对于二叉搜索树的便利情况，可能比较难完成")
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
    #[allow(unused_variables, unused_mut,dead_code)]
    fn insert(&mut self, mut ptr: NonNull<Node>) -> bool {
        match self.0 {
            Some(mut root_ptr) => {
                // 这个地方还是与之前的完全一致，只是尝试加多一个判断
                let root = unsafe { root_ptr.as_mut() } ;
                if root_ptr.as_ptr() as usize == ptr.as_ptr() as usize {
                   false
                }
                else {
                    let ret = match ptr < root_ptr {
                        true => &mut root.l,
                        false=> &mut root.r,
                    }.insert(ptr);
                    root.update();
                    self.rotate();
                    ret 
                }
            },
            None => {
                // create a node and insert it
                self.0 = Some(ptr);
                *unsafe { ptr.as_mut() } = Node {
                    l: Tree(None),
                    r: Tree(None),
                    h: 1, 
                };
                true
            },
        }
        // if let Some(mut root_ptr) = self.0 {
        //     // 插入结点
        //     let root = unsafe { root_ptr.as_mut() };
        //     if ptr < root_ptr {
        //         &mut root.l
        //     } else {
        //         &mut root.r
        //     }
        //     .insert(ptr);
        //     root.update();
        //     self.rotate();
        // } else {
        //     // 新建结点
        //     self.0 = Some(ptr);
        //     *unsafe { ptr.as_mut() } = Node {
        //         l: Tree(None),
        //         r: Tree(None),
        //         h: 1,
        //     };
        // }
    }
    
    /// 从地址池中获取一个单位的地址, 并且返回这个地址
    #[allow(dead_code, unused_variables,unused_mut)]
    fn delete(&mut self) -> Option<usize>{
        /* 
        根据需求，此处需要实现的子模块包括，通过 左右子树中的最小高度导航到 到最近的叶子结点，然后再进行 删除节点操作
        删除节点操作的时候，由于当前操作在叶子节点处产生，因此不需要考虑额外信息，直接将其删除即可
        */
        match self.0 {
            None => { None } ,
            Some(mut root_ptr) => {
                let root = unsafe { root_ptr.as_mut() };
                let ret = match (root.l.0.is_none(), root.r.0.is_none()) {
                    (true, true) => {
                        // this is the root node (and it's leaf) => clean itself
                        let node = self.0.unwrap().as_ptr() as usize;
                        self.0 = None;
                        return Some(node);
                    },
                    (true, false) => {
                        match root.l.height() {
                            1 => {
                                let node = root.r.0.unwrap().as_ptr() as usize;
                                root.l = Tree(None);
                                Some(node)
                            },
                            _ => root.r.delete(),
                        }
                    },
                    (false, true) => {
                        match root.r.height() {
                            1 => {
                                let node = root.r.0.unwrap().as_ptr() as usize;
                                root.r = Tree(None);
                                Some(node)
                            },
                            _ => root.l.delete(),
                        }
                    },
                    (false, false) => {
                        // 这个地方实际上主要目的在于减少代码量...但是反而带来了可读性的降低
                        match (root.l.height() < root.r.height(), core::cmp::min(root.l.height(), root.r.height())) {
                            (true, 1) => {
                                let node = root.l.0.unwrap().as_ptr() as usize;
                                root.l = Tree(None);
                                Some(node)
                            },
                            (true, _) => root.l.delete(),
                            (false, 1) => {
                                let node = root.r.0.unwrap().as_ptr() as usize;
                                root.r = Tree(None);
                                Some(node)
                            },
                            (false, _) => root.l.delete(),
                        }
                    },
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

#[allow(unused_variables,dead_code, unused_mut)]
#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    const LIST_NUM: usize = 6;

    impl Node {
        fn new() -> Self {
            Self {
                l: Tree(None), 
                r: Tree(None),
                h:1, 
            }
        }
    }

    impl Tree {
        
        fn insert_from_list(&mut self, list: Vec<NonNull<Node>>) {
            for item in 0..list.len() {
                self.insert(list[item]);
            }
        }

        #[allow(dead_code)]
        fn preorder_traversal_seq(&self) -> Vec<usize> {
            let mut ret = Vec::new();
            ret.push(self.0.unwrap().as_ptr() as usize);
            if let Some(root_ptr) = self.0 {
                let root = unsafe { root_ptr.as_ref() };
                match &root.l {
                    Tree(None) => (),
                    tree => ret.append(&mut tree.preorder_traversal_seq()),
                }
                match &root.r {
                    Tree(None) => (),
                    tree => ret.append(&mut tree.preorder_traversal_seq()),
                }
            }
            else {
                println!("failure");
            }
            return ret;
        }
        
        fn level_traversal(&self, list: [NonNull<Node>; LIST_NUM]) -> Vec<VecDeque<usize>> {
            let mut res:Vec<VecDeque<usize>> = Vec::new();
            let mut queue:VecDeque<&Tree> = VecDeque::new();

            match self.0 {
                None => return res,
                Some(_) => queue.push_back(self),
            }

            while !queue.is_empty() {
                let node_size = queue.len();
                let mut this_line = VecDeque::new();

                for _ in 0..node_size {
                    let tree = queue.pop_front().expect("msg 1");
                    // println!("{:?}", mapping_addr_and_number(list, &tree));
                    // this_line.push_back(tree.0.expect("msg 5").as_ptr() as usize);

                    // let a = list.iter().enumerate().filter(|(v, node)| {
                    //     node.as_ptr() as usize == tree.0.expect("msg 6").as_ptr() as usize 
                    // });
                    this_line.push_back(mapping_addr_and_number(list, &tree));

                    unsafe {
                        match tree.0.expect("msg 2").as_ref().l {
                            Tree(None) => (),
                            ref tree => queue.push_back(tree),
                        }       
                        match tree.0.expect("msg 3").as_ref().r {
                            Tree(None) => (),
                            ref tree => queue.push_back(tree),
                        }       
                    }
                }
                res.push(this_line);
            }

            res

            // let ret = Vec::new();
            // let queue = Vec::new();
            // if let Some(node) = self.0 {
            //     queue.push(self);
            //     let split = Tree::new();
            //     split.h = 0;
            //     queue.push(split);
            // }
            // while let Some(node) = queue.pop() {
            //     let node = node.0 {
            //         Some(node) => node,
            //     }
            //     // let node = unsafe { node.0.unwrap().as_ref() };
            //     if node.h == 0 {
            //         ret.push(queue);
            //         queue.clear();
            //     }
            // }
        }
    }

    #[test]
    fn test_for_insert_l() { 
        let mut a = Node::new();let mut b = Node::new();let mut c = Node::new();let mut d = Node::new();let mut e = Node::new(); let mut f = Node::new();
        let ptr1: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut a as *mut _) }; let ptr2: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut b as *mut _) }; let ptr3: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut c as *mut _) }; let ptr4: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut d as *mut _) }; let ptr5: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut e as *mut _) }; let ptr6: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut f as *mut _) };
        // println!("ptr1  {:p}", ptr1);println!("ptr2  {:p}", ptr2);println!("ptr3  {:p}", ptr3);println!("ptr4  {:p}", ptr4);println!("ptr5  {:p}", ptr5);println!("ptr6  {:p}", ptr6);

        let mut tree = Tree(None);
        let list = [ptr1, ptr2, ptr3, ptr4, ptr5, ptr6];


        let insertion_sequence = Vec::from([ptr1, ptr2, ptr3]);
        tree.insert_from_list(insertion_sequence);
        
        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list); 
        let level_vec = tree.level_traversal(list);
        assert_eq!(level_vec, vec![vec![2], vec![1,3]]);
        // println!("{:?}", level_vec);
        assert_eq!(mapping_addr_and_number(list, &tree), 2);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().l }), 1);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().r }), 3);
    }

    #[test]
    fn test_for_insert_rl() { 
        let mut a = Node::new();let mut b = Node::new();let mut c = Node::new();let mut d = Node::new();let mut e = Node::new(); let mut f = Node::new();
        let ptr1: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut a as *mut _) }; let ptr2: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut b as *mut _) }; let ptr3: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut c as *mut _) }; let ptr4: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut d as *mut _) }; let ptr5: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut e as *mut _) }; let ptr6: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut f as *mut _) };
        // println!("ptr1  {:p}", ptr1);println!("ptr2  {:p}", ptr2);println!("ptr3  {:p}", ptr3);println!("ptr4  {:p}", ptr4);println!("ptr5  {:p}", ptr5);println!("ptr6  {:p}", ptr6);

        let mut tree = Tree(None);
        let list = [ptr1, ptr2, ptr3, ptr4, ptr5, ptr6];


        let insertion_sequence = Vec::from([ptr1, ptr2, ptr3]);
        tree.insert_from_list(insertion_sequence);
        
        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list); 
        let level_vec = tree.level_traversal(list);
        assert_eq!(level_vec, vec![vec![2], vec![1,3]]);
        // println!("{:?}", level_vec);
        assert_eq!(mapping_addr_and_number(list, &tree), 2);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().l }), 1);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().r }), 3);
    }
    
    #[test]
    fn test_for_insert_r() { 
        let mut a = Node::new();let mut b = Node::new();let mut c = Node::new();let mut d = Node::new();let mut e = Node::new(); let mut f = Node::new();
        let ptr1: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut a as *mut _) }; let ptr2: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut b as *mut _) }; let ptr3: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut c as *mut _) }; let ptr4: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut d as *mut _) }; let ptr5: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut e as *mut _) }; let ptr6: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut f as *mut _) };
        // println!("ptr1  {:p}", ptr1);println!("ptr2  {:p}", ptr2);println!("ptr3  {:p}", ptr3);println!("ptr4  {:p}", ptr4);println!("ptr5  {:p}", ptr5);println!("ptr6  {:p}", ptr6);

        let mut tree = Tree(None);
        let list = [ptr1, ptr2, ptr3, ptr4, ptr5, ptr6];


        let insertion_sequence = Vec::from([ptr3, ptr2, ptr1]);
        tree.insert_from_list(insertion_sequence);
        
        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list); 
        let level_vec = tree.level_traversal(list);
        assert_eq!(level_vec, vec![vec![2], vec![1,3]]);
        // println!("{:?}", level_vec);

        assert_eq!(mapping_addr_and_number(list, &tree), 2);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().l }), 1);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().r }), 3);
    }

    #[test]
    fn test_for_insert_lr() { 
        let mut a = Node::new();let mut b = Node::new();let mut c = Node::new();let mut d = Node::new();let mut e = Node::new(); let mut f = Node::new();
        let ptr1: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut a as *mut _) }; let ptr2: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut b as *mut _) }; let ptr3: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut c as *mut _) }; let ptr4: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut d as *mut _) }; let ptr5: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut e as *mut _) }; let ptr6: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut f as *mut _) };
        // println!("ptr1  {:p}", ptr1);println!("ptr2  {:p}", ptr2);println!("ptr3  {:p}", ptr3);println!("ptr4  {:p}", ptr4);println!("ptr5  {:p}", ptr5);println!("ptr6  {:p}", ptr6);

        let mut tree = Tree(None);
        let list = [ptr1, ptr2, ptr3, ptr4, ptr5, ptr6];


        let insertion_sequence = Vec::from([ptr3, ptr1, ptr2]);
        tree.insert_from_list(insertion_sequence);
        
        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list); 
        let level_vec = tree.level_traversal(list);
        assert_eq!(level_vec, vec![vec![2], vec![1,3]]);
        // println!("{:?}", level_vec);

        assert_eq!(mapping_addr_and_number(list, &tree), 2);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().l }), 1);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().r }), 3);
    }

    #[test]
    fn test_for_delete_l() {
        let mut a = Node::new();let mut b = Node::new();let mut c = Node::new();let mut d = Node::new();let mut e = Node::new(); let mut f = Node::new();
        let ptr1: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut a as *mut _) }; let ptr2: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut b as *mut _) }; let ptr3: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut c as *mut _) }; let ptr4: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut d as *mut _) }; let ptr5: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut e as *mut _) }; let ptr6: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut f as *mut _) };
        // println!("ptr1  {:p}", ptr1);println!("ptr2  {:p}", ptr2);println!("ptr3  {:p}", ptr3);println!("ptr4  {:p}", ptr4);println!("ptr5  {:p}", ptr5);println!("ptr6  {:p}", ptr6);

        let mut tree = Tree(None);
        let list = [ptr1, ptr2, ptr3, ptr4, ptr5, ptr6];

        let insertion_sequence = Vec::from([ptr2, ptr1, ptr3, ptr4]);
        tree.insert_from_list(insertion_sequence);

        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list);

        tree.delete();

        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list);
        
        assert_eq!(mapping_addr_and_number(list, &tree), 3);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().l }), 2);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().r }), 4);
    }

    #[test]
    #[allow(unused_variables)]
    fn test_for_delete_r() {
        let mut a = Node::new();let mut b = Node::new();let mut c = Node::new();let mut d = Node::new();let mut e = Node::new(); let mut f = Node::new();
        let ptr1: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut a as *mut _) }; let ptr2: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut b as *mut _) }; let ptr3: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut c as *mut _) }; let ptr4: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut d as *mut _) }; let ptr5: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut e as *mut _) }; let ptr6: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut f as *mut _) };
        println!("ptr1  {:p}", ptr1);println!("ptr2  {:p}", ptr2);println!("ptr3  {:p}", ptr3);println!("ptr4  {:p}", ptr4);println!("ptr5  {:p}", ptr5);println!("ptr6  {:p}", ptr6);

        let mut tree = Tree(None);
        let list = [ptr1, ptr2, ptr3, ptr4, ptr5, ptr6];

        let insertion_sequence = Vec::from([ptr3, ptr4, ptr2, ptr1]);
        tree.insert_from_list(insertion_sequence);

        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list);

        tree.delete();

        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list);
        
        assert_eq!(mapping_addr_and_number(list, &tree), 2);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().l }), 1);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().r }), 3);
    }

    #[test]
    fn test_for_delete_lr() {
        let mut a = Node::new();let mut b = Node::new();let mut c = Node::new();let mut d = Node::new();let mut e = Node::new(); let mut f = Node::new();
        let ptr1: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut a as *mut _) }; let ptr2: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut b as *mut _) }; let ptr3: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut c as *mut _) }; let ptr4: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut d as *mut _) }; let ptr5: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut e as *mut _) }; let ptr6: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut f as *mut _) };
        println!("ptr1  {:p}", ptr1);println!("ptr2  {:p}", ptr2);println!("ptr3  {:p}", ptr3);println!("ptr4  {:p}", ptr4);println!("ptr5  {:p}", ptr5);println!("ptr6  {:p}", ptr6);

        let mut tree = Tree(None);
        let list = [ptr1, ptr2, ptr3, ptr4, ptr5, ptr6];

        let insertion_sequence = Vec::from([ptr3, ptr4, ptr1, ptr2]);
        tree.insert_from_list(insertion_sequence);

        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list);

        tree.delete();

        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list);
        
        assert_eq!(mapping_addr_and_number(list, &tree), 2);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().l }), 1);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().r }), 3);
    }

    #[test]
    fn test_for_delete_rl() {
        let mut a = Node::new();let mut b = Node::new();let mut c = Node::new();let mut d = Node::new();let mut e = Node::new(); let mut f = Node::new();
        let ptr1: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut a as *mut _) }; let ptr2: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut b as *mut _) }; let ptr3: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut c as *mut _) }; let ptr4: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut d as *mut _) }; let ptr5: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut e as *mut _) }; let ptr6: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut f as *mut _) };
        // println!("ptr1  {:p}", ptr1);println!("ptr2  {:p}", ptr2);println!("ptr3  {:p}", ptr3);println!("ptr4  {:p}", ptr4);println!("ptr5  {:p}", ptr5);println!("ptr6  {:p}", ptr6);

        let mut tree = Tree(None);
        let list = [ptr1, ptr2, ptr3, ptr4, ptr5, ptr6];

        let insertion_sequence = Vec::from([ptr2, ptr1, ptr4, ptr3]);
        tree.insert_from_list(insertion_sequence);

        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list);

        tree.delete();

        // print_level_traversal(&tree, list);
        // print_pre_inorder_traversal(&tree, list);
        
        assert_eq!(mapping_addr_and_number(list, &tree), 3);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().l }), 2);
        assert_eq!(mapping_addr_and_number(list, unsafe { &tree.0.unwrap().as_ref().r }), 4);
    }

    #[test] 
    fn test_for_delete_root() {
        let mut a = Node::new();let mut b = Node::new();let mut c = Node::new();let mut d = Node::new();let mut e = Node::new(); let mut f = Node::new();
        let ptr1: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut a as *mut _) }; let ptr2: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut b as *mut _) }; let ptr3: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut c as *mut _) }; let ptr4: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut d as *mut _) }; let ptr5: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut e as *mut _) }; let ptr6: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut f as *mut _) };
        // println!("ptr1  {:p}", ptr1);println!("ptr2  {:p}", ptr2);println!("ptr3  {:p}", ptr3);println!("ptr4  {:p}", ptr4);println!("ptr5  {:p}", ptr5);println!("ptr6  {:p}", ptr6);

        let mut tree = Tree(None);
        let list = [ptr1, ptr2, ptr3, ptr4, ptr5, ptr6];

        let insertion_sequence = Vec::from([ptr2, ptr1, ptr4, ptr3]);
        tree.insert_from_list(insertion_sequence);

        println!("0x{:X}", tree.delete().unwrap() );
        println!("0x{:X}", tree.delete().unwrap() );
        // tree.delete();
        
        assert_eq!(tree.preorder_traversal_seq().len(), 2);
        
        // print_level_traversal(&tree, list);
        println!("0x{:X}", tree.delete().unwrap() );
        
        assert_eq!(tree.preorder_traversal_seq().len(), 1);
        // print_level_traversal(&tree, list);
        
        println!("0x{:X}", tree.delete().unwrap() );
        assert_eq!(tree.0, Tree(None).0);

        assert_eq!(tree.delete(), None);
    }

    fn print_level_traversal(tree: &Tree, list: [NonNull<Node>; LIST_NUM]) {
        println!("================================================================");
        let level_vec = tree.level_traversal(list);
        println!("{:?}", level_vec);
    }
    
    fn print_pre_inorder_traversal(tree: &Tree, list: [NonNull<Node>; LIST_NUM]) {
        println!("================================================================");
        let pre_vec = tree.preorder_traversal_seq();
        print_vec(pre_vec, list);
    }

    fn print_vec(out: Vec<usize>, list: [NonNull<Node>; LIST_NUM]) {
        for i in 0..out.len() {
            // println!("===");
            // println!("{}", out[i]);
            list.iter().enumerate().for_each(|(v, ptr)| {
                // println!("{}", ptr.as_ptr() as usize);
                if ptr.as_ptr() as usize == out[i] {
                    print!("{}  ", v + 1);
                };
            });
        }
        println!();
    }

    // 让序号更加直观
    fn mapping_addr_and_number(list: [NonNull<Node> ; LIST_NUM], addr: &Tree) -> usize {
        let ptr_for_node = addr.0.expect("msg 6").as_ptr() as usize;
        for i in 0..list.len() {
            if ptr_for_node == list[i].as_ptr() as usize {
                return i + 1;
            }
        }
        100 
    }

    #[test]
    #[allow(unused_variables)]
    fn test_if_cmp_could_using_to_compare_two_nonnull() {
        let mut a = Node::new();let mut b = Node::new();let mut c = Node::new();let mut d = Node::new();let mut e = Node::new(); let mut f = Node::new();
        let ptr1: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut a as *mut _) }; let ptr2: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut b as *mut _) }; let ptr3: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut c as *mut _) }; let ptr4: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut d as *mut _) }; let ptr5: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut e as *mut _) }; let ptr6: NonNull<Node> = unsafe { NonNull::new_unchecked(&mut f as *mut _) };

        let a = core::cmp::min(ptr1, ptr2);
        let b = core::cmp::min(ptr3, ptr2);
        // println!("{ptr1:?}, {ptr2:?}");
        // println!("{ptr3:?}, {ptr2:?}");
        // println!("{:?}", a);
        // println!("{:?}", b);
        assert_eq!(a, ptr1);
        assert_eq!(b, ptr2);
    }    

}