//test context switch overhead
use crate::*;

use std::thread;
const CTX_ITER: usize = ITERATIONS * 1000;
/// Measures the time to switch between two kernel threads.
/// Calls `do_ctx_inner` multiple times to perform the actual operation
pub fn do_ctx() -> Result<(), &'static str> {
    let mut tries: f64 = 0.0;
    let mut max: f64 = core::f64::MIN;
    let mut min: f64 = core::f64::MAX;
    let mut vec: Vec<f64> = Vec::with_capacity(TRIES);
    print_header(TRIES, CTX_ITER);

    for i in 0..TRIES {
        let lat = do_ctx_inner(i + 1, TRIES)?;

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
            "ctx_test diff is too big: {:.3} ({:.3} - {:.3}) {}",
            max - min,
            max,
            min,
            T_UNIT
        );
    }
    let stats = calculate_stats(&vec).ok_or("couldn't calculate stats")?;
    printlninfo!("Context switch result: ({})", T_UNIT);
    printlninfo!("{:?}", stats);
    printlninfo!("This test does not have an equivalent test in LMBench");
    Ok(())
}

fn do_ctx_inner(th: usize, nr: usize /*, child_core: u8 */) -> Result<f64, &'static str> {
    let start_hpet = get_timer_value()?;
    let end_hpet: f64;
    let overhead_end_hpet: f64;

    // Spawning "overhead" threads
    let taskref1 = thread::spawn(overhead_task);
    let taskref2 = thread::spawn(overhead_task);

    // Waiting for "overhead" threads to complete
    taskref1.join().expect("Task 1 failed");
    taskref2.join().expect("Task 2 failed");

    // Timer simulation for overhead tasks (replace with actual timer logic)
    let overhead_end_hpet = get_timer_value()?;
    println!("Overhead end HPET: {:?}", overhead_end_hpet);

    // Spawning "yield" threads
    let taskref3 = thread::spawn(yield_task);
    let taskref4 = thread::spawn(yield_task);

    // Waiting for "yield" threads to complete
    taskref3.join().expect("Task 3 failed");
    taskref4.join().expect("Task 4 failed");

    end_hpet = get_timer_value()?;

    let delta_overhead = overhead_end_hpet - start_hpet;
    let delta_hpet = end_hpet - overhead_end_hpet - delta_overhead;
    let delta_time_avg = delta_hpet / (CTX_ITER * 2) as f64; //*2 because each thread yields ITERATION number of times
    printlninfo!(
        "ctx_switch_test_inner ({}/{}): total_overhead -> {:.3} {} , {:.3} total_time -> {:.3} {}",
        th,
        nr,
        delta_overhead,
        T_UNIT,
        delta_hpet,
        delta_time_avg,
        T_UNIT
    );
    Ok(delta_time_avg)
}

// 模拟 sys_yield 功能
fn sys_yield() {
    thread::yield_now();
}

// overhead 执行的函数
fn overhead_task() {
    println!("hello, this is overhead test");
}

// yield 执行的函数
fn yield_task() {
    for _ in 0..CTX_ITER {
        sys_yield();
    }
}
