#![allow(unused, dead_code)]
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};



//！just using for avl tree test, trying to allocate and deallocate node by a different ways to make more node in one line at the same times
use customizable_buddy::{BuddyAllocator, BuddyError, UsizeBuddy, AvlBuddy};
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

const DIVICE_PIECE :usize = 2;
/// 256 MiB
static mut MEMORY: [Page; 65536] = [Page::ZERO; 65536];

/// init the allocate of heap -> which buddy_allocater have a lof of block inside
fn init_deallocate() -> Result<Allocator<12>, BuddyError> {
    // 创建全局分配器
    let mut allocator = Allocator::<12>::new();
    // 从本机操作系统获取一块内存给程序
    let ptr = NonNull::new(unsafe { MEMORY.as_mut_ptr() }).unwrap();
    let len = core::mem::size_of_val(unsafe { &MEMORY });
    // 使用最小阶数和初始地址初始化程序
    allocator.init(12, ptr);
    unsafe { allocator.transfer(ptr, len) };
    Ok(allocator)
}

fn allocate(mut allocator: Allocator<12>) -> Result<(), BuddyError> {
    let mut blocks = [null_mut::<Page>(); 65536];
    let layout = Layout::new::<Page>();
    for block in blocks.iter_mut() {
        let (ptr, size) = allocator.allocate_type::<Page>()?;
        debug_assert_eq!(layout.size(), size);
        *block = ptr.as_ptr();
    }
    Ok(())
}

fn deallocate(mut allocator: Allocator<12>) -> Result<(), BuddyError> {
    let mut blocks = [null_mut::<Page>(); 65536];
    let layout = Layout::new::<Page>();
    for block in blocks.iter_mut() {
        // 将指向对应page大小的地址指定给对应指针(将地址从buddyLine内删除)
        let (ptr, size) = allocator.allocate_type::<Page>()?;
        debug_assert_eq!(layout.size(), size);
        *block = ptr.as_ptr();
    }
    for i in 0..DIVICE_PIECE {
        let mut cnt = 0 ;
        for j in 0..blocks.len() {
            if cnt == i {
                allocator.deallocate(NonNull::new(blocks[j]).unwrap(), layout.size());
                blocks[i] = null_mut();
            }
            cnt += 1;
            if cnt == DIVICE_PIECE {
                cnt = 0;
            }
        }
    }
    Ok(())
}

fn criterion_benchmark_init(c: &mut Criterion) {
    c.bench_function("init", |b| {
        b.iter(||  init_deallocate());
    });
}

fn criterion_benchmark_allocate(c: &mut Criterion) {
    c.bench_function("allocate", |b| {
        allocate(init_deallocate().expect("msg 1: init error")).expect("msg 2: allocate error")
    });
}

fn criterion_benchmark_deallocate(c: &mut Criterion) {
    c.bench_function("deallocate", |b| {
        deallocate(init_deallocate().expect("msg 1: init error")).expect("msg 2: deallocate error")
    });
}

criterion_group!(benches, criterion_benchmark_init, criterion_benchmark_allocate, criterion_benchmark_deallocate);
criterion_main!(benches);