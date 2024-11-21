/// Perform null syscall benchmark
use crate::*;

pub fn do_null() -> Result<(), &'static str> {
    let mut tries: f64 = 0.0;
    let mut max: f64 = f64::MIN;
    let mut min: f64 = f64::MAX;
    let mut vec = Vec::with_capacity(TRIES);

    let overhead_ct = get_timing_overhead()?;
    print_header(TRIES, ITERATIONS * 1_000);

    for i in 0..TRIES {
        let lat = do_null_inner(overhead_ct, i + 1, TRIES)?;

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
    let err = (lat * 10.0) / 100.0; // Allowable error margin
    if max - lat > err || lat - min > err {
        printlnwarn!(
            "null_test diff is too big: {:.3} ({:.3} - {:.3}) {}",
            max - min,
            max,
            min,
            T_UNIT
        );
    }

    printlninfo!("Null syscall test completed successfully.");
    let stats = calculate_stats(&vec).ok_or("couldn't calculate stats")?;

    printlninfo!("{:?}", stats);
    printlninfo!("This test is equivalent to `lat_syscall null` in LMBench");
    Ok(())
}

/// Internal function that actually calculates the time for null syscall.
/// Measures this by calling `libredox::call::getpid()` of the current task.
fn do_null_inner(overhead_ct: f64, th: usize, nr: usize) -> Result<f64, &'static str> {
    let start_hpet = get_timer_value()? as f64;
    let mut end_hpet: f64;

    let tmp_iterations = ITERATIONS * 1_000;
    for _ in 0..tmp_iterations {
        let _mypid = libredox::call::getpid();
    }
    end_hpet = get_timer_value()? as f64;

    let mut delta_hpet = end_hpet - start_hpet;
    if delta_hpet < overhead_ct {
        printlnwarn!(
            "Ignore overhead for null because overhead({}) > diff({})",
            overhead_ct,
            delta_hpet
        );
    } else {
        delta_hpet -= overhead_ct;
    }

    let delta_time_avg = delta_hpet / (tmp_iterations as f64);

    printlninfo!(
        "null_test_inner ({}/{}): overhead {:.3}, {:.3} total_time -> {:.3}",
        th,
        nr,
        overhead_ct,
        delta_hpet,
        delta_time_avg
    );

    Ok(delta_time_avg)
}
