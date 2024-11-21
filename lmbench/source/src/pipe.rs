use crate::*;
use nix::unistd::{close, fork, pipe, ForkResult};
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;
use std::process::Command;
use std::time::Instant;

const PIPE_ITER: usize = ITERATIONS * 100; // 测试循环次数

pub fn do_pipe() -> Result<(), &'static str> {
    // 创建两个管道

    let mut tries: f64 = 0.0;
    let mut max: f64 = core::f64::MIN;
    let mut min: f64 = core::f64::MAX;
    let mut vec = Vec::with_capacity(TRIES);
    let mut lat = 0.0;

    let overhead_ct = get_timing_overhead()?;
    print_header(TRIES, PIPE_ITER);

    for i in 0..TRIES {
        let (p1_read, p1_write) = create_pipe();
        let (p2_read, p2_write) = create_pipe();
        // 创建子进程
        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                // 子进程: 关闭父进程使用的管道端
                close(p1_write).unwrap();
                close(p2_read).unwrap();

                // 子进程运行 writer
                writer(p1_read, p2_write);

                // 退出子进程
                std::process::exit(0);
            }
            Ok(ForkResult::Parent { child }) => {
                // 父进程: 关闭子进程使用的管道端
                close(p1_read).unwrap();
                close(p2_write).unwrap();

                // 测试通信延迟
                lat = measure_latency(overhead_ct, i + 1, TRIES, p1_write, p2_read, PIPE_ITER)?;

                // 清理子进程
                Command::new("kill")
                    .arg("-9")
                    .arg(format!("{}", nix::unistd::getpid()))
                    .output()
                    .unwrap();
            }
            Err(err) => {
                eprintln!("Fork failed: {}", err);
                std::process::exit(1);
            }
        }

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
            "lat_pipe_test diff is too big: {:.3} ({:.3} - {:.3}) {}",
            max - min,
            max,
            min,
            T_UNIT
        );
    }
    let stats = calculate_stats(&vec)
        .ok_or("couldn't calculate stats")
        .unwrap();

    printlninfo!("LAT_PIPE result: ({})", T_UNIT);
    printlninfo!("{:?}", stats);
    printlninfo!("This test is equivalent to `lat_pipe` in LMBench");
    Ok(())
}

// 创建管道
fn create_pipe() -> (i32, i32) {
    let (read_fd, write_fd) = pipe().expect("Failed to create pipe");
    (read_fd, write_fd)
}

// 测量管道通信延迟
fn measure_latency(
    overhead_ct: f64,
    th: usize,
    nr: usize,
    write_fd: i32,
    read_fd: i32,
    iterations: usize,
) -> Result<f64, &'static str> {
    let mut dummy_sum = 0;
    let mut buf = [0u8; 1]; // 单字节缓冲区

    // 打开文件描述符为 Rust 标准库的 File
    let mut writer = unsafe { File::from_raw_fd(write_fd) };
    let mut reader = unsafe { File::from_raw_fd(read_fd) };

    // 记录起始时间
    let start_hpet = get_timer_value()?;

    for _ in 0..iterations {
        // 父进程写数据到 p1
        writer.write_all(&buf).expect("Failed to write to pipe");
        writer.flush().unwrap();

        // 父进程从 p2 读取数据
        reader
            .read_exact(&mut buf)
            .expect("Failed to read from pipe");
        dummy_sum += buf[0] as u64;
    }

    // 记录结束时间
    let end_hpet = get_timer_value()?;
    let delta_hpet = end_hpet - start_hpet - overhead_ct;
    let delta_time = delta_hpet;
    let delta_time_avg = delta_time / PIPE_ITER as f64;

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

// 子进程逻辑
fn writer(read_fd: i32, write_fd: i32) {
    let mut buf = [0u8; 1];
    let mut reader = unsafe { File::from_raw_fd(read_fd) };
    let mut writer = unsafe { File::from_raw_fd(write_fd) };

    loop {
        match reader.read_exact(&mut buf) {
            Ok(_) => {
                writer.write_all(&buf).expect("Failed to write to pipe");
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                println!("Writer detected EOF, exiting...");
                break;
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}

pub fn do_pipe_bandwidth(packet_size: usize, total_size: usize) -> Result<(), &'static str> {
    let mut tries: f64 = 0.0;
    let mut max: f64 = core::f64::MIN;
    let mut min: f64 = core::f64::MAX;
    let mut vec = Vec::with_capacity(TRIES);

    let overhead_ct = get_timing_overhead()?;
    print_header(TRIES, PIPE_ITER);

    for i in 0..TRIES {
        let bandwidth = measure_bandwidth(overhead_ct, i + 1, TRIES, packet_size, total_size)?;
        tries += bandwidth;
        vec.push(bandwidth);

        if bandwidth > max {
            max = bandwidth;
        }
        if bandwidth < min {
            min = bandwidth;
        }
    }

    let avg_bandwidth = tries / TRIES as f64;
    let err = (avg_bandwidth * 10.0 * THRESHOLD_ERROR_RATIO as f64) / 100.0;

    if max - avg_bandwidth > err || avg_bandwidth - min > err {
        printlnwarn!(
            "bw_pipe_test diff is too big: {:.3} ({:.3} - {:.3}) {}",
            max - min,
            max,
            min,
            T_UNIT
        );
    }

    let stats = calculate_stats(&vec).ok_or("couldn't calculate stats")?;
    printlninfo!("BW_PIPE result: ({})", T_UNIT);
    printlninfo!("{:?}", stats);
    let microseconds_in_one_second = 1_000_000.0; // 10^6
    let bytes_in_one_mb = 2_f64.powi(20);         // 2^20
    // Convert bytes/us to MB/s
    let mb_per_s = (avg_bandwidth * microseconds_in_one_second) / bytes_in_one_mb;
    printlninfo!("Average bandwidth: {:.3}MB / sec", mb_per_s );
    printlninfo!("This test is equivalent to `bw_pipe` in LMBench");
    Ok(())
}

fn measure_bandwidth(
    overhead_ct: f64,
    th: usize,
    nr: usize,
    packet_size: usize,
    total_size: usize,
) -> Result<f64, &'static str> {
    let mut buf = vec![0u8; packet_size];
    let mut total_transferred = 0;

    // 创建管道
    let (read_fd, write_fd) = create_pipe();

    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            // 子进程: 关闭读取端
            close(read_fd).unwrap();

            let mut writer = unsafe { File::from_raw_fd(write_fd) };
            loop {
                match writer.write(&buf) {
                    Ok(written) => {
                        if written == 0 {
                            break; // 写入结束，退出循环
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                        // 遇到 BrokenPipe 错误，正常退出循环
                        break;
                    }
                    Err(e) => {
                        // 对其他错误的处理，保持程序稳定
                        eprintln!("Unexpected error during write: {:?}", e);
                        break;
                    }
                }
            }
            std::process::exit(0);
        }
        Ok(ForkResult::Parent { child }) => {
            // 父进程: 关闭写入端
            close(write_fd).unwrap();

            // 父进程读取数据
            let mut reader = unsafe { File::from_raw_fd(read_fd) };
            let start_time = get_timer_value()?;

            while total_transferred < total_size {
                let read = reader.read(&mut buf).expect("Read from pipe failed");
                if read == 0 {
                    break;
                }
                total_transferred += read;
            }

            let end_time = get_timer_value()?;
            close(read_fd).unwrap();

            // 清理子进程
            Command::new("kill")
                .arg("-9")
                .arg(format!("{}", child))
                .output()
                .unwrap();

            let elapsed_time = end_time - start_time - overhead_ct;
            let bandwidth = total_size as f64 / elapsed_time; // 以字节每时间单位衡量带宽

            printlninfo!(
                "bw_pipe_test_inner ({}/{}): total_bytes: {}, time: {:.3}, bandwidth: {:.3} bytes/{}",
                th,
                nr,
                total_transferred,
                elapsed_time,
                bandwidth,
                T_UNIT
            );

            Ok(bandwidth)
        }
        Err(err) => {
            eprintln!("Fork failed: {}", err);
            std::process::exit(1);
        }
    }
}
