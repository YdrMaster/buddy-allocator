# 伙伴分配器

[![Latest version](https://img.shields.io/crates/v/customizable-buddy.svg)](https://crates.io/crates/customizable-buddy)
[![Documentation](https://docs.rs/customizable-buddy/badge.svg)](https://docs.rs/customizable-buddy)
![license](https://img.shields.io/github/license/YdrMaster/buddy-allocator)
[![CI](https://github.com/YdrMaster/buddy-allocator/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/YdrMaster/buddy-allocator/actions)
[![issue](https://img.shields.io/github/issues/YdrMaster/buddy-allocator)](https://github.com/YdrMaster/buddy-allocator/issues)

伙伴分配器。

用法参见[性能测试示例](/examples/bench.rs)和[自定义实现示例](/examples/debug.rs)。

与常见的实现的区别：

- 定义了伙伴行\*的接口，支持替换伙伴查找算法。内置 usize bitmap 和单链表实现，可以自定义实现；
- 定义了寡头行的不同接口。伙伴分配器定义为一个二叉森林，可以有多个根。所有根的集合是最顶层的一行，因为没有相邻伙伴合并的行为，称为寡头行；
- 使用 `transfer` 将内存块转移给分配器，使用 `snatch` 从分配器取出内存块，动态控制分配器管理的内存块；
- 不包含锁或加锁版本的接口，要实现 `GlobalAlloc` 或 `Allocator` 需要自定义可变性管理方式；
  > 单线程的应用建议不加锁，用某种 `Cell` 描述可变性；
- 带有多种分配接口，分别基于一个类型、一个布局描述或原始参数分配；

---

> **NOTICE** “行”是 háng。意为伙伴分配器管理的同样大小的那一组块。
