//！just using for avl tree test, trying to allocate and deallocate node by a different ways to make more node in one line at the same times
use customizable_buddy::{AvlBuddy, BuddyAllocator, BuddyError, UsizeBuddy};
use std::{
    alloc::Layout,
    ptr::{null_mut, NonNull},
    time::Instant,
};

type Allocator<const N: usize> = BuddyAllocator<N, UsizeBuddy, AvlBuddy>;

#[repr(C, align(4096))]
struct Page([u8; 4096]);

impl Page {
    const ZERO: Self = Self([0; 4096]);
}

const DIVICE_PIECE: usize = 2;
/// 256 MiB
static mut MEMORY: [Page; 65536] = [Page::ZERO; 65536];

fn main() -> Result<(), BuddyError> {
    // 创建全局分配器
    let mut allocator = Allocator::<12>::new();
    // 从本机操作系统获取一块内存给程序
    let ptr = NonNull::new(unsafe { MEMORY.as_mut_ptr() }).unwrap();
    let len = core::mem::size_of_val(unsafe { &MEMORY });
    // 使用最小阶数和初始地址初始化程序
    allocator.init(12, ptr);
    println!(
        "MEMORY: {:#x}..{:#x}",
        ptr.as_ptr() as usize,
        ptr.as_ptr() as usize + len
    );
    // 计时
    let t = Instant::now();
    // 将地址空间放入分配器进行分配【此时生成的结果应该是默认分配情况下的】
    unsafe { allocator.transfer(ptr, len) };
    println!("transfer {:?}", t.elapsed());

    assert_eq!(len, allocator.capacity());
    assert_eq!(len, allocator.free());

    println!(
        "
BEFORE
{allocator:#x?}"
    );

    // 创建页面大小的指针？
    let mut blocks = [null_mut::<Page>(); 65536];
    let layout = Layout::new::<Page>();
    let t = Instant::now();
    for block in blocks.iter_mut() {
        // 将指向对应page大小的地址指定给对应指针(将地址从buddyLine内删除)
        let (ptr, size) = allocator.allocate_type::<Page>()?;
        debug_assert_eq!(layout.size(), size);
        *block = ptr.as_ptr();
    }
    // let (ptr, size) = allocator.allocate_type::<Page>()?;
    // debug_assert_eq!(layout.size(), size);
    // blocks[i] = ptr.as_ptr();
    // for i in 0..DIVICE_PIECE {
    //     // 对于所有的blocks我们便利四次，但是由于LinkedList 和 AVL 都在同等情况下进行测试，因此不会因为循环过多的次数影响到输出的结果
    //     let mut cnt = 0;
    //     for j in 0..50 {
    //         if cnt == i {
    //             let (ptr, size) = allocator.allocate_type::<Page>()?;
    //             debug_assert_eq!(layout.size(), size);
    //             blocks[j] = ptr.as_ptr();
    //         }
    //         cnt += 1;
    //         if cnt == DIVICE_PIECE {
    //             cnt = 0;
    //         }
    //     }
    // }
    let ta = t.elapsed();

    // 呈现出全部都被收回的结果
    println!(
        "
EMPTY
{allocator:#x?}"
    );

    assert_eq!(len, allocator.capacity());
    assert_eq!(len - blocks.len() * layout.size(), allocator.free());

    println!("here");
    let t = Instant::now();
    // for block in blocks.iter_mut() {
    //     // 释放指针所指向的空间给分配器进行调配
    //     allocator.deallocate(NonNull::new(*block).unwrap(), layout.size());
    //     *block = null_mut();
    //     // println!("{allocator:#x?}");
    // }
    for i in 0..DIVICE_PIECE {
        let mut cnt = 0;
        for j in 0..blocks.len() {
            if cnt == i {
                allocator.deallocate(NonNull::new(blocks[j]).unwrap(), layout.size());
                blocks[i] = null_mut();
            }
            // println!("{:#x?} , cnt: {cnt:?}, i: {i:?}", blocks[j]);
            cnt += 1;
            if cnt == DIVICE_PIECE {
                cnt = 0;
            }
        }
        println!("{allocator:#x?}");
    }
    let td = t.elapsed();

    assert_eq!(len, allocator.capacity());
    assert_eq!(len, allocator.free());

    println!(
        "
AFTER
{allocator:#x?}"
    );

    println!(
        "allocate   {:?} ({} times)",
        ta / blocks.len() as u32,
        blocks.len()
    );
    println!(
        "deallocate {:?} ({} times)",
        td / blocks.len() as u32,
        blocks.len()
    );

    Ok(())
}
