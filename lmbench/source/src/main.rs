pub mod ctx;
pub mod fs;
pub mod mmap;
pub mod null;
pub mod pipe;

use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
// extern crate libredox;
pub const ITERATIONS: usize = 100;
pub const TRIES: usize = 10;
pub const T_UNIT: &str = "micro_sec";
/// Macro for printing informational messages
#[macro_export]
macro_rules! printlninfo {
    ($fmt:expr) => (println!($fmt));
    ($fmt:expr, $($arg:tt)*) => (println!($fmt, $($arg)*));
}

/// Macro for printing warning messages
#[macro_export]
macro_rules! printlnwarn {
    ($fmt:expr) => (println!($fmt));
    ($fmt:expr, $($arg:tt)*) => (println!($fmt, $($arg)*));
}

/// Main entry point
fn main() {
    let mut args = std::env::args().skip(1); // 跳过程序名称
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "debug" => {
                debug_test_main();
            }
            "null" => {
                let _ = null::do_null();
            }
            "ctx" => {
                let _ = ctx::do_ctx();
            }
            "bw_file_rd" => {
                let _ = fs::do_fs_read(true);
                let _ = fs::do_fs_read(false);
            }
            "lat_fs" => {
                let _ = fs::do_fs_create_del();
                let _ = fs::do_fs_delete();
            }
            "lat_pipe" => {
                pipe::do_pipe();
            }
            "lat_mmap" => {
                // 检查是否有额外参数
                if let Some(param) = args.next() {
                    // 尝试将参数转换为整数
                    if let Ok(extra_param) = param.parse::<usize>() {
                        let _ = mmap::do_memory_map(extra_param * 1024);
                    } else {
                        println!("Invalid parameter for lat_mmap: {}", param);
                        std::process::exit(1);
                    }
                } else {
                    // 没有额外参数时调用默认版本
                    println!("No parameter for lat_mmap, using default:4KB mapping size");
                    let _ = mmap::do_memory_map(4096);
                }
            }
            "bw_mmap_rd" => {
                // 获取大小参数
                if let Some(size_arg) = args.next() {
                    if let Ok(size_kb) = size_arg.parse::<usize>() {
                        let size = size_kb * 1024; // 转换为字节
                        if let Some(mode) = args.next() {
                            match mode.as_str() {
                                "mmap_only" => {
                                    create_testfile("testfile", size)
                                        .expect("Failed to create test file");
                                    let _ = mmap::do_mmap_only(size);
                                }
                                "open2close" => {
                                    create_testfile("testfile", size)
                                        .expect("Failed to create test file");
                                    let _ = mmap::do_open2close(size);
                                }
                                _ => {
                                    println!("Invalid mode for bw_mmap_rd: {}", mode);
                                    std::process::exit(1);
                                }
                            }
                        } else {
                            println!("No mode provided for bw_mmap_rd (expected: mmap_only or open2close)");
                            std::process::exit(1);
                        }
                    } else {
                        println!("Invalid size parameter for bw_mmap_rd: {}", size_arg);
                        std::process::exit(1);
                    }
                } else {
                    println!("No size parameter for bw_mmap_rd (expected: size in KB)");
                    std::process::exit(1);
                }
            }
            "bw_pipe" => {
                // 获取大小参数
                if let Some(msg_size_arg) = args.next() {
                    if let Ok(msg_size_kb) = msg_size_arg.parse::<usize>() {
                        let msg = msg_size_kb * 1024; // 转换为字节
                        if let Some(total_size_arg) = args.next() {
                            if let Ok(total_size_mb) = total_size_arg.parse::<usize>() {
                                let total_size = total_size_mb * 1024 * 1024; // 转换为字节
                                let _ = pipe::do_pipe_bandwidth(msg, total_size);
                            } else {
                                println!("Invalid size parameter for bw_pipe: {}", total_size_arg);
                                std::process::exit(1);
                            }
                        } else {
                            println!("Invalid total_size parameter for bw_pipe ");
                            std::process::exit(1);
                        }
                    } else {
                        println!("Invalid msg_size parameter for bw_pipe: {}", msg_size_arg);
                        std::process::exit(1);
                    }
                } else {
                    println!("No size parameter for bw_pipe (expected: size in KB)");
                    std::process::exit(1);
                }
            }
            _ => {
                println!("Unknown command: {}", arg);
            }
        }
    }
}

/// Debug entry function
fn debug_test_main() {}

/// Print the header of the test
fn print_header(tries: usize, iterations: usize) {
    printlninfo!("========================================");
    printlninfo!("Time unit : {}", T_UNIT);
    printlninfo!("Iterations: {}", iterations);
    printlninfo!("Tries     : {}", tries);
    printlninfo!("========================================");
}

/// Measures the overhead of using the timer
fn get_timing_overhead() -> Result<f64, &'static str> {
    const TRIES: u64 = 10;
    let mut tries: f64 = 0.0;
    let mut max: f64 = f64::MIN;
    let mut min: f64 = f64::MAX;

    for _ in 0..TRIES {
        let overhead = get_timing_overhead_inner()?;
        tries += overhead;
        if overhead > max {
            max = overhead;
        }
        if overhead < min {
            min = overhead;
        }
    }

    let overhead = tries / TRIES as f64;
    printlninfo!(
        "get_timer_value() overhead is {:.3} microseconds, original tries {:.3}",
        overhead,
        tries
    );
    Ok(overhead)
}

/// Internal function that calculates timer overhead
fn get_timing_overhead_inner() -> Result<f64, &'static str> {
    const ITERATIONS: usize = 10_000;
    let start_time = get_timer_value()?;
    let mut _tmp_time = 0.0;

    for _ in 0..ITERATIONS {
        _tmp_time = get_timer_value()?;
    }
    let end_time = get_timer_value()?;

    let delta_time = end_time - start_time;
    let delta_time_avg = delta_time / ITERATIONS as f64;

    printlninfo!(
        "get_timer_value() overhead is {} microseconds ({} microsecods / iteration)",
        delta_time,
        delta_time_avg
    );

    Ok(delta_time_avg)
}

use std::collections::BTreeMap;
use std::fmt;
use std::vec::Vec;

pub const THRESHOLD_ERROR_RATIO: u64 = 1;

use std::time::{SystemTime, UNIX_EPOCH};
// use libc::timespec;
// Return the mtime raw value
pub fn get_timer_value() -> Result<f64, &'static str> {
    match libredox::call::clock_gettime(4) {
        Ok(ts) => {
            let microseconds = (ts.tv_sec * 1_000_000) + (ts.tv_nsec / 1_000);
            Ok(microseconds as f64)
            // let nanoseconds = (ts.tv_sec * 1_000_000_000) + ts.tv_nsec;
            // Ok(nanoseconds as u64)
        }
        Err(_) => {
            println!("Error getting time");
            Err("Error getting time")
        }
    }
}

pub struct Stats {
    pub min: f64,
    pub p_25: f64,
    pub median: f64,
    pub p_75: f64,
    pub max: f64,
    pub mode: f64,
    pub mean: f64,
    pub std_dev: f64,
}

impl fmt::Debug for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Stats \n 
        min:     {:.3} \n 
        p_25:    {:.3} \n 
        median:  {:.3} \n 
        p_75:    {:.3} \n 
        max:     {:.3} \n 
        mode:    {:.3} \n 
        mean:    {:.3} \n 
        std_dev: {:.3} \n",
            self.min,
            self.p_25,
            self.median,
            self.p_75,
            self.max,
            self.mode,
            self.mean,
            self.std_dev
        )
    }
}

/// Helper function to calculate statistics of a provided dataset
pub fn calculate_stats(vec: &Vec<f64>) -> Option<Stats> {
    let mean;
    let median;
    let mode;
    let p_75;
    let p_25;
    let min;
    let max;
    let var;
    let std_dev;

    if vec.is_empty() {
        return None;
    }

    let len = vec.len();

    {
        // calculate average
        let sum: f64 = vec.iter().sum();
        mean = sum as f64 / len as f64;
    }

    {
        // calculate median
        let mut vec2 = vec.clone();
        // vec2.sort();
        vec2.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = len / 2;
        let i_75 = len * 3 / 4;
        let i_25 = len * 1 / 4;

        median = vec2[mid];
        p_25 = vec2[i_25];
        p_75 = vec2[i_75];
        min = vec2[0];
        max = vec2[len - 1];
    }

    {
        // calculate sample variance
        let mut diff_sum: f64 = 0.0;
        for &val in vec {
            let x = val as f64;
            diff_sum += (x - mean).powi(2);
        }

        var = diff_sum / len as f64;
        std_dev = var.sqrt();
    }

    {
        // 使用 BTreeMap
        let mut values: BTreeMap<i64, usize> = BTreeMap::new();
        for val in vec {
            let key = (val * 100.0) as i64; // 将浮点数放大并转换为整数
            values.entry(key).and_modify(|v| *v += 1).or_insert(1);
        }

        // 计算众数
        let tmp = *values
            .iter()
            .max_by(|(_k1, v1), (_k2, v2)| v1.cmp(v2))
            .unwrap()
            .0;

        // 转换回浮点数
        mode = tmp as f64 / 100.0;
    }

    Some(Stats {
        min,
        p_25,
        median,
        p_75,
        max,
        mode,
        mean,
        std_dev,
    })
}

pub fn create_testfile(filename: &str, size: usize) -> Result<(), Box<dyn Error>> {
    // 打开文件（如果存在会覆盖）
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);

    // 向文件写入指定大小的数据
    let buffer = vec![0u8; 1024 * 1024]; // 1MB 的缓冲区
    let mut remaining = size;

    while remaining > 0 {
        let write_size = if remaining >= buffer.len() {
            buffer.len()
        } else {
            remaining
        };
        writer.write_all(&buffer[..write_size])?;
        remaining -= write_size;
    }

    writer.flush()?;
    printlninfo!("Test file '{}' created with size {} bytes.", filename, size);
    Ok(())
}
