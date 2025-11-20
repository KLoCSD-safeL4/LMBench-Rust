use crate::*;

use libc::{c_void, fstat, mmap, munmap, open, MAP_SHARED, O_RDONLY, PROT_READ};
use std::ffi::CString;
use std::ptr;
use std::time::{Duration, Instant};

const PAGE_SIZE: usize = 4096; // 页大小
const PAGEFAULT_ITER: usize = ITERATIONS * 100; // 页面错误迭代次数
                                                // 初始化：创建符合测试大小的文件并返回文件描述符、映射地址和大小
fn initialize(file: &str, test_size: usize) -> (i32, *mut u8, usize) {
    // 创建测试文件并写入测试数据
    let mut data = vec![0u8; test_size];
    std::fs::write(file, &data).expect("Failed to create test file");

    let c_file = CString::new(file).expect("Failed to create CString");
    // 打开文件
    let fd = unsafe { open(c_file.as_ptr(), O_RDONLY) };
    if fd < 0 {
        panic!("Failed to open file: {}", file);
    }

    // 内存映射
    let addr = unsafe { mmap(ptr::null_mut(), test_size, PROT_READ, MAP_SHARED, fd, 0) };

    (fd, addr as *mut u8, test_size)
}

// mmap 开销测试
fn mmap_overhead(addr: *mut u8, size: usize) -> (f64, *mut u8) {
    let start = get_timer_value().unwrap();

    for _ in 0..PAGEFAULT_ITER {
        // 解除映射
        unsafe { munmap(addr as *mut c_void, size) };
        // 映射内存
        let addr = unsafe { mmap(ptr::null_mut(), size, PROT_READ, MAP_SHARED, -1, 0) };
    }
    let end = get_timer_value().unwrap();
    let duration = end - start;
    (duration, addr as *mut u8)
}

// 页面错误测试
fn do_pagefault_inner(
    overhead: f64,
    th: usize,
    nr: usize,
    fd: i32,
    size: usize,
    addr: *mut u8,
) -> f64 {
    let npages = size / PAGE_SIZE;
    let mut sum = 0;

    let start: f64;
    let end: f64;
    let delta: f64;

    start = get_timer_value().unwrap();
    for _ in 0..PAGEFAULT_ITER {
        for i in 0..npages {
            unsafe {
                let page = addr.add(i * PAGE_SIZE);
                sum += *page as i32;
            }
        }
        let addr = addr as *mut c_void;
        unsafe {
            if munmap(addr as *mut c_void, size) != 0 {
                panic!("Failed to munmap memory");
            }
        }
        let addr = unsafe { mmap(ptr::null_mut(), size, PROT_READ, MAP_SHARED, fd, 0) };
    }
    end = get_timer_value().unwrap();
    delta = end - start - overhead;
    let delta_avg = delta / PAGEFAULT_ITER as f64;
    printlninfo!(
        "page_fault_test_inner ({}/{}): hpet {:.3} , overhead {:.3}, {:.3} total_time -> {:.3} {}",
        th,
        nr,
        delta,
        overhead,
        delta,
        delta_avg,
        T_UNIT
    );
    delta_avg
}

// 主入口
pub fn do_pagefault(file: &str, test_size: usize) {
    let mut tries: f64 = 0.0;
    let (fd, addr, size) = initialize(file, test_size);

    println!("File mapped to memory. Size: {} bytes", size);

    let overhead_ct = get_timing_overhead().unwrap();
    print_header(TRIES, PAGEFAULT_ITER);

    let mut overhead = 0.0;
    for _ in 0..TRIES {
        let (mmap_time, _) = mmap_overhead(addr, size);
        overhead += mmap_time;
    }
    overhead = overhead / (PAGEFAULT_ITER * TRIES) as f64 - overhead_ct;
    println!("Average mmap overhead: {:.3} ms", overhead);

    let mut max: f64 = core::f64::MIN;
    let mut min: f64 = core::f64::MAX;
    let mut vec = Vec::with_capacity(TRIES);

    for i in 0..TRIES {
        let lat = do_pagefault_inner(overhead_ct,i,TRIES, fd, size, addr);
        tries += lat;
        vec.push(lat);

        if lat > max {
            max = lat;
        }
        if lat < min {
            min = lat;
        }
    }
    let lat = tries / TRIES as f64;

    // println!("Average page fault latency: {:.3} us", avg_time-overhead);

    // We expect the maximum and minimum to be within 10*THRESHOLD_ERROR_RATIO % of the mean value
    let err = (lat * 10.0 * THRESHOLD_ERROR_RATIO as f64) / 100.0;
    if max - lat > err || lat - min > err {
        printlnwarn!(
            "pagefault_test diff is too big: {:.3} ({:.3} - {:.3}) {}",
            max - min,
            max,
            min,
            T_UNIT
        );
    }
    let stats = calculate_stats(&vec).ok_or("couldn't calculate stats").unwrap();

    printlninfo!("PAGE_FAULT result: ({})", T_UNIT);
    printlninfo!("{:?}", stats);
    printlninfo!("This test is equivalent to `lat_pagefault` in LMBench");
    // 清理资源
    unsafe {
        munmap(addr as *mut c_void, size);
        libc::close(fd);
    }

    // 删除测试文件
    std::fs::remove_file(file).expect("Failed to remove test file");
}
