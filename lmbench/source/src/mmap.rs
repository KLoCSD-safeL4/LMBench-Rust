use crate::*;
use libc::{
    close, mmap, munmap, open, MAP_ANONYMOUS, MAP_PRIVATE, MAP_SHARED, O_RDONLY, PROT_READ,
    PROT_WRITE,
};

use std::ffi::CString;
use std::ptr;

const MMAP_ITER: usize = ITERATIONS * 100;
/// Measures the time to create and destroy a mapping.
/// Calls `do_memory_map_inner` multiple times to perform the actual operation
pub fn do_memory_map(size: usize) -> Result<(), &'static str> {
    let mut tries: f64 = 0.0;
    let mut max: f64 = core::f64::MIN;
    let mut min: f64 = core::f64::MAX;
    let mut vec = Vec::with_capacity(TRIES);

    let overhead_ct = get_timing_overhead()?;
    print_header(TRIES, MMAP_ITER);

    for i in 0..TRIES {
        let lat = do_memory_map_inner(overhead_ct, i + 1, TRIES, size)?;

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
    // We expect the maximum and minimum to be within 10*THRESHOLD_ERROR_RATIO % of the mean value
    let err = (lat * 10.0 * THRESHOLD_ERROR_RATIO as f64) / 100.0;
    if max - lat > err || lat - min > err {
        printlnwarn!(
            "memory_map_test diff is too big: {:.3} ({:.3} - {:.3}) {}",
            max - min,
            max,
            min,
            T_UNIT
        );
    }
    let stats = calculate_stats(&vec).ok_or("couldn't calculate stats")?;

    printlninfo!("MEMORY MAP result: ({})", T_UNIT);
    printlninfo!("{:?}", stats);
    printlninfo!("This test is equivalent to `lat_mmap` in LMBench");
    Ok(())
}

/// Internal function that actually calculates the time to create and destroy a memory mapping.
/// Measures this by continually allocating and dropping `MappedPages`.
fn do_memory_map_inner(
    overhead_ct: f64,
    th: usize,
    nr: usize,
    mapsize: usize,
) -> Result<f64, &'static str> {
    let MAPPING_SIZE: usize = mapsize;

    let start_hpet: f64;
    let end_hpet: f64;
    let delta_hpet: f64;

    start_hpet = get_timer_value()?;
    for _ in 0..MMAP_ITER {
        // 调用 mmap 映射一块内存
        let addr = unsafe {
            mmap(
                ptr::null_mut(),
                MAPPING_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_ANONYMOUS | MAP_PRIVATE,
                -1,
                0,
            )
        };

        unsafe {
            let data = addr as *mut u8;
            *data = 0xFF; // 写入一个字节
        }

        // 调用 munmap 解除映射
        let result = unsafe { munmap(addr, MAPPING_SIZE) };
    }

    end_hpet = get_timer_value()?;

    delta_hpet = end_hpet - start_hpet - overhead_ct;
    let delta_time = delta_hpet;
    let delta_time_avg = delta_time / MMAP_ITER as f64;
    printlninfo!(
        "memory_map_test_inner ({}/{}): hpet {:.3} , overhead {:.3}, {:.3} total_time -> {:.3} {}",
        th,
        nr,
        delta_hpet,
        overhead_ct,
        delta_time,
        delta_time_avg,
        T_UNIT
    );

    Ok(delta_time_avg)
}

/// mmap_only 测试
pub fn do_mmap_only(size: usize) -> Result<(), &'static str> {
    let mut tries: f64 = 0.0;
    let mut max: f64 = core::f64::MIN;
    let mut min: f64 = core::f64::MAX;
    let mut vec = Vec::with_capacity(TRIES);

    let overhead_ct = get_timing_overhead()?;
    print_header(TRIES, MMAP_ITER);

    for i in 0..TRIES {
        let lat = do_mmap_only_inner(overhead_ct, i + 1, TRIES, size)?;
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
    let err = (lat * 10.0 * THRESHOLD_ERROR_RATIO as f64) / 100.0;
    if max - lat > err || lat - min > err {
        printlnwarn!(
            "mmap_only_test diff is too big: {:.3} ({:.3} - {:.3}) {}",
            max - min,
            max,
            min,
            T_UNIT
        );
    }
    let stats = calculate_stats(&vec).ok_or("couldn't calculate stats")?;

    printlninfo!("MMAP_ONLY result: ({})", T_UNIT);
    printlninfo!("{:?}", stats);
    Ok(())
}

fn do_mmap_only_inner(
    overhead_ct: f64,
    th: usize,
    nr: usize,
    size: usize,
) -> Result<f64, &'static str> {
    let filename = "testfile"; // 替换为你的测试文件路径
    let c_filename = CString::new(filename).expect("CString::new failed");

    // 打开文件
    let fd = unsafe { open(c_filename.as_ptr(), O_RDONLY) };
    if fd < 0 {
        return Err("Failed to open file");
    }

    let start_hpet = get_timer_value()?;

    for _ in 0..ITERATIONS {
        // mmap 文件内容
        let addr = unsafe { mmap(ptr::null_mut(), size, PROT_READ, MAP_SHARED, fd, 0) };

        if addr == libc::MAP_FAILED {
            unsafe { close(fd) };
            return Err("Failed to mmap file");
        }

        // 模拟读取
        unsafe {
            let data = addr as *const u8;
            for i in 0..size {
                let _ = *data.offset(i as isize); // 逐字节读取
            }
        }

        // 解除映射
        let result = unsafe { munmap(addr, size) };
        if result != 0 {
            unsafe { close(fd) };
            return Err("Failed to munmap");
        }
    }

    let end_hpet = get_timer_value()?;
    unsafe { close(fd) };

    let delta_hpet = end_hpet - start_hpet - overhead_ct;
    let delta_time_avg = delta_hpet / ITERATIONS as f64;

    printlninfo!(
        "mmap_only_test_inner ({}/{}): hpet {:.3}, overhead {:.3}, {:.3} total_time -> {:.3} {}",
        th,
        nr,
        delta_hpet,
        overhead_ct,
        delta_hpet,
        delta_time_avg,
        T_UNIT
    );

    Ok(delta_time_avg)
}

/// open2close 测试
pub fn do_open2close(size: usize) -> Result<(), &'static str> {
    let mut tries: f64 = 0.0;
    let mut max: f64 = core::f64::MIN;
    let mut min: f64 = core::f64::MAX;
    let mut vec = Vec::with_capacity(TRIES);

    let overhead_ct = get_timing_overhead()?;
    print_header(TRIES, MMAP_ITER);

    for i in 0..TRIES {
        let lat = do_open2close_inner(overhead_ct, i + 1, TRIES, size)?;
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
    let err = (lat * 10.0 * THRESHOLD_ERROR_RATIO as f64) / 100.0;
    if max - lat > err || lat - min > err {
        printlnwarn!(
            "open2close_test diff is too big: {:.3} ({:.3} - {:.3}) {}",
            max - min,
            max,
            min,
            T_UNIT
        );
    }
    let stats = calculate_stats(&vec).ok_or("couldn't calculate stats")?;

    printlninfo!("OPEN2CLOSE result: ({})", T_UNIT);
    printlninfo!("{:?}", stats);
    Ok(())
}

fn do_open2close_inner(
    overhead_ct: f64,
    th: usize,
    nr: usize,
    size: usize,
) -> Result<f64, &'static str> {
    let filename = "testfile"; // 替换为你的测试文件路径
    let c_filename = CString::new(filename).expect("CString::new failed");

    let start_hpet = get_timer_value()?;

    for _ in 0..ITERATIONS {
        // 打开文件
        let fd = unsafe { open(c_filename.as_ptr(), O_RDONLY) };
        if fd < 0 {
            return Err("Failed to open file");
        }

        // mmap 文件内容
        let addr = unsafe { mmap(ptr::null_mut(), size, PROT_READ, MAP_SHARED, fd, 0) };

        if addr == libc::MAP_FAILED {
            unsafe { close(fd) };
            return Err("Failed to mmap file");
        }

        // 模拟读取
        unsafe {
            let data = addr as *const u8;
            for i in 0..size {
                let _ = *data.offset(i as isize); // 逐字节读取
            }
        }

        // 解除映射
        let result = unsafe { munmap(addr, size) };
        if result != 0 {
            unsafe { close(fd) };
            return Err("Failed to munmap");
        }

        // 关闭文件
        unsafe { close(fd) };
    }

    let end_hpet = get_timer_value()?;

    let delta_hpet = end_hpet - start_hpet - overhead_ct;
    let delta_time_avg = delta_hpet / ITERATIONS as f64;

    printlninfo!(
        "open2close_test_inner ({}/{}): hpet {:.3}, overhead {:.3}, {:.3} total_time -> {:.3} {}",
        th,
        nr,
        delta_hpet,
        overhead_ct,
        delta_hpet,
        delta_time_avg,
        T_UNIT
    );

    Ok(delta_time_avg)
}
