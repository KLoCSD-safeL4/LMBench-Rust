const FS_ITER: usize = ITERATIONS * 10;
const WRITE_BUF_SIZE: usize = 1024 * 1024;
const WRITE_BUF: [u8; WRITE_BUF_SIZE] = [65; WRITE_BUF_SIZE];
const FILE_SIZE: usize = 4096;
const MB: u64 = 1024 * 1024;
const KB: u64 = 1024;
const NANOSECOND: usize = 1_000_000_000;
const MICROSECOND: usize = 1_000_000;
use crate::*;
use std::fs::{remove_file, File, OpenOptions};
use std::io::{self, Read, Write};
/// tests for fs
pub fn do_fs_read(with_open: bool) -> Result<(), &'static str> {
    let fsize_kb = 4;
    printlninfo!("File size     : {} KB", fsize_kb);
    printlninfo!("Read buf size : {} KB", FILE_SIZE);
    printlninfo!("========================================");

    //Used to measure the overhead of the timer
    let overhead_ct = get_timing_overhead()?;

    do_fs_read_with_size(overhead_ct, fsize_kb, with_open)?;
    if with_open {
        printlninfo!("This test is equivalent to `bw_file_rd open2close` in LMBench");
    } else {
        printlninfo!("This test is equivalent to `bw_file_rd io_only` in LMBench");
    }
    Ok(())
}

fn do_fs_read_with_size(
    overhead_ct: f64,
    fsize_kb: usize,
    with_open: bool,
) -> Result<(), &'static str> {
    let mut tries: f64 = 0.0;
    let mut tries_mb: f64 = 0.0;
    let mut tries_kb: f64 = 0.0;
    let mut max: f64 = core::f64::MIN;
    let mut min: f64 = core::f64::MAX;
    let fsize_b = fsize_kb * KB as usize;
    let mut vec = Vec::new();

    let filename = "tmp.txt\n".to_string(); // file size 4 KB.

    // we can use `mk_tmp_file()` because it is outside of the loop
    mk_tmp_file(&filename, fsize_b).expect("Cannot create a file");

    for i in 0..TRIES {
        let (lat, tput_mb, tput_kb) = if with_open {
            do_fs_read_with_open_inner(&filename, overhead_ct, i + 1, TRIES, fsize_b)
                .expect("Error in read_open inner()")
        } else {
            do_fs_read_only_inner(&filename, overhead_ct, i + 1, TRIES, fsize_b)
                .expect("Error in read_only inner()")
        };

        tries += lat;
        tries_mb += tput_mb;
        tries_kb += tput_kb;
        vec.push(tput_kb);

        if lat > max {
            max = lat;
        }
        if lat < min {
            min = lat;
        }
    }
    remove_file(filename).expect("Cannot delete a file");
    let stats = calculate_stats(&vec).ok_or("couldn't calculate stats")?;

    let lat = tries / TRIES as f64;
    let tput_mb = tries_mb / TRIES as f64;
    let tput_kb = tries_kb / TRIES as f64;
    let err = (lat * 10.0 + lat * THRESHOLD_ERROR_RATIO as f64) / 10.0;
    if max - lat > err || lat - min > err {
        printlnwarn!(
            "test diff is too big: {:.3} ({:.3} - {:.3}) {}",
            max - min,
            max,
            min,
            T_UNIT
        );
    }

    print_header(TRIES, FS_ITER);
    printlninfo!(
        "{} for {:.3} KB: {:.3} {} {:.3} MB/sec {:.3} KB/sec",
        if with_open {
            "READ WITH OPEN"
        } else {
            "READ ONLY"
        },
        fsize_kb,
        lat,
        T_UNIT,
        tput_mb,
        tput_kb
    );
    printlninfo!("{:?}", stats);
    Ok(())
}

fn do_fs_read_with_open_inner(
    filename: &str,
    overhead_ct: f64,
    th: usize,
    nr: usize,
    fsize: usize,
) -> Result<(f64, f64, f64), &'static str> {
    let start_hpet: f64;
    let end_hpet: f64;
    let mut _dummy_sum: u64 = 0;

    if fsize != FILE_SIZE {
        return Err("File size is not alligned");
    }

    let mut buf = [0u8; FILE_SIZE];
    start_hpet = get_timer_value()?;
    let end_fd = 0;
    for _ in 0..FS_ITER {
        // 打开文件（读写模式，不存在则创建）
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)
            .unwrap();

        // 读取文件
        let read_size = file.read(&mut buf).unwrap();
        _dummy_sum += read_size as u64;
    }
    end_hpet = get_timer_value()?;

    let delta_hpet = end_hpet - start_hpet - overhead_ct;
    let delta_time = delta_hpet;
    let delta_time_avg = delta_time / FS_ITER as f64;

    let mb_per_sec = (WRITE_BUF_SIZE * MICROSECOND) as f64 / (MB as f64 * delta_time_avg); // prefer this
    let kb_per_sec = (WRITE_BUF_SIZE * MICROSECOND) as f64 / (KB as f64 * delta_time_avg);
    // for i in end_fd - FS_ITER + 1..=end_fd {
    //     sys_close(i as usize);
    // }
    printlninfo!(
        "read_with_open_inner ({}/{}): {:.3} total_time -> {:.3} {} {:.3} MB/sec {:.3} KB/sec (ignore: {})",
        th,
        nr,
        delta_time,
        delta_time_avg,
        T_UNIT,
        mb_per_sec,
        kb_per_sec,
        _dummy_sum
    );

    Ok((delta_time_avg, mb_per_sec, kb_per_sec))
}

/// Internal function that actually calculates the time to read a file.
/// This function read the file and sums up the read charachters in each chunk.
/// This is performed to be compatible with `LMBench`
fn do_fs_read_only_inner(
    filename: &str,
    overhead_ct: f64,
    th: usize,
    nr: usize,
    fsize: usize,
) -> Result<(f64, f64, f64), &'static str> {
    let start_hpet: f64;
    let end_hpet: f64;

    let mut _dummy_sum: u64 = 0;
    if fsize != FILE_SIZE {
        return Err("File size is not alligned");
    }
    let mut buf = [0u8; FILE_SIZE];

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(filename)
        .unwrap();

    start_hpet = get_timer_value()?;
    for _ in 0..FS_ITER {
        // let nr_read = sys_read(fd as usize, &mut buf);
        let nr_read = file.read(&mut buf).unwrap();
        _dummy_sum += nr_read as u64;
    }
    end_hpet = get_timer_value()?;
    drop(file);

    let delta_hpet = end_hpet - start_hpet - overhead_ct;
    let delta_time = delta_hpet;
    println!("delete_time: {:.3} us", delta_time);
    let delta_time_avg = delta_time / FS_ITER as f64;

    println!("the delete_time_avg: {}", delta_time_avg);
    let mb_per_sec = (WRITE_BUF_SIZE * MICROSECOND) as f64 / (MB as f64 * delta_time_avg); // prefer this
    let kb_per_sec = (WRITE_BUF_SIZE * MICROSECOND) as f64 / (KB as f64 * delta_time_avg);

    printlninfo!(
        "read_only_inner ({}/{}): {:.3} total_time -> {:.3} {} {:.3} MB/sec {:.3} KB/sec (ignore: {})",
        th,
        nr,
        delta_time,
        delta_time_avg,
        T_UNIT,
        mb_per_sec,
        kb_per_sec,
        _dummy_sum
    );

    Ok((delta_time_avg, mb_per_sec, kb_per_sec))
}

fn mk_tmp_file(filename: &str, sz: usize) -> Result<(), &'static str> {
    if sz > WRITE_BUF_SIZE {
        return Err("Cannot test because the file size is too big");
    } else if sz != FILE_SIZE {
        return Err("Cannot test because the file size is not the same as the test file");
    }

    // create test file
    let write_buf = [64u8; FILE_SIZE];
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(filename)
        .unwrap();
    file.write(&write_buf).expect("Cannot write to a file");
    drop(file);

    Ok(())
}

pub fn do_fs_create_del() -> Result<(), &'static str> {
    let fsizes_b = [1024_usize, 4096, 8 * 1024];

    let overhead_ct = get_timing_overhead()?;

    print_header(TRIES, FS_ITER);
    printlninfo!("SIZE(KB)    Iteration    created(files/s)     time(ns/file)");
    for fsize_b in fsizes_b.iter() {
        do_fs_create_del_inner(*fsize_b, overhead_ct)?;
    }
    printlninfo!("This test is equivalent to file create in `lat_fs` in LMBench");

    Ok(())
}

fn do_fs_create_del_inner(fsize_b: usize, overhead_ct: f64) -> Result<(), &'static str> {
    let start_hpet_create: f64;
    let end_hpet_create: f64;
    let create_iter = FS_ITER;

    // check if we have enough data to write. We use just const data to avoid unnecessary overhead
    if fsize_b > WRITE_BUF_SIZE {
        return Err("Cannot test because the file size is too big");
    }

    let wbuf = &WRITE_BUF[0..fsize_b];

    // Measuring loop - create
    start_hpet_create = get_timer_value()?;

    for i in 0..create_iter {
        let filename = format!("tmp_{}_{}.txt", fsize_b, i);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)
            .expect("Cannot create a file");
        file.write(&wbuf).expect("Cannot write to a file");
        drop(file);
    }
    end_hpet_create = get_timer_value()?;

    let delta_hpet_create = end_hpet_create - start_hpet_create - overhead_ct;
    let delta_time_create = delta_hpet_create;

    let files_per_time = (create_iter * MICROSECOND) as f64 / delta_time_create;

    printlninfo!(
        "{:8}    {:9}    {:16.3}    {:16.3}",
        fsize_b / KB as usize,
        create_iter,
        files_per_time,
        delta_time_create / create_iter as f64
    );

    // delete all test files avoid Influence the next round of testing

    for i in 0..create_iter {
        let filename = format!("tmp_{}_{}.txt", fsize_b, i);
        remove_file(filename).expect("DELETE_FILE_FAIL");
    }
    Ok(())
}

pub fn do_fs_delete() -> Result<(), &'static str> {
    let fsizes_b = [1024_usize, 4096, 2 * 4096];

    let overhead_ct = get_timing_overhead()?;

    print_header(TRIES, FS_ITER);
    printlninfo!("SIZE(KB)    Iteration    deleted(files/s)    time(ns/file)");
    for fsize_b in fsizes_b.iter() {
        do_fs_delete_inner(*fsize_b, overhead_ct)?;
    }
    printlninfo!("This test is equivalent to file delete in `lat_fs` in LMBench");
    Ok(())
}

/// Internal function that actually calculates the time to delete to a file.
/// Within the measurin section it remove the given file reference from current working directory
/// Prior to measuring files are created and their referecnes are added to a vector
fn do_fs_delete_inner(fsize_b: usize, overhead_ct: f64) -> Result<(), &'static str> {
    let start_hpet_create: f64;
    let end_hpet_create: f64;
    let del_iter = FS_ITER;

    // check if we have enough data to write. We use just const data to avoid unnecessary overhead
    if fsize_b > WRITE_BUF_SIZE {
        return Err("Cannot test because the file size is too big");
    }

    // delete existing files. To make sure that the file creation below succeeds.

    let wbuf = &WRITE_BUF[0..fsize_b];

    // Non measuring loop for file create
    for i in 0..del_iter {
        let filename = format!("tmp_{}_{}.txt", fsize_b, i);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)
            .expect("Cannot create a file");
        file.write(&wbuf).expect("Cannot write to a file");
        drop(file);
    }

    start_hpet_create = get_timer_value()?;
    // Measuring loop file delete
    for i in 0..del_iter {
        let filename = format!("tmp_{}_{}.txt", fsize_b, i);
        remove_file(filename);
    }
    end_hpet_create = get_timer_value()?;

    let delta_hpet_delete = end_hpet_create - start_hpet_create - overhead_ct;
    let delta_time_delete = delta_hpet_delete;

    let files_per_time = (del_iter * MICROSECOND) as f64 / delta_time_delete;

    printlninfo!(
        "{:8}    {:9}    {:16.3}    {:16.3}",
        fsize_b / KB as usize,               // file size in KB
        del_iter,                            // number of files
        files_per_time,                      // files per designated time
        delta_time_delete / del_iter as f64  // average time to delete a file
    );
    Ok(())
}
