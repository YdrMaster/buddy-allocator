﻿use customizable_buddy::{BuddyAllocator, BuddyError, AvlBuddy, UsizeBuddy};
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
    // 将地址空间放入分配器进行分配
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
    let ta = t.elapsed();

    // 由于将等同于分配空间大小的页面全部收回
    println!(
        "
EMPTY
{allocator:#x?}"
    );

    // 感觉这个地方应该 有问题, 不应该总容量不变
    assert_eq!(len, allocator.capacity());
    assert_eq!(len - blocks.len() * layout.size(), allocator.free());

    let t = Instant::now();
    for block in blocks.iter_mut() {
        // 释放指针所指向的空间给分配器进行调配
        allocator.deallocate(NonNull::new(*block).unwrap(), layout.size());
        *block = null_mut();
        // println!("{allocator:#x?}");
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
