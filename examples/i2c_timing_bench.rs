//! Benchmark `calc_timing_params_full` execution time on hardware using the DWT cycle counter.
//!
//! Calls the function N times, collects per-call tick counts, then reports
//! min, max, mean, and standard deviation.
//!
//! Run with:
//!   cargo run --example i2c_timing_bench --features stm32h503

#![deny(warnings)]
#![no_main]
#![no_std]

mod utilities;

use cortex_m_rt::entry;
use log::info;
use stm32h5xx_hal::{
    dwt::DwtExt,
    i2c::calc_timing_bench,
    pac,
    prelude::*,
};

// const N: usize = 100;
const N: usize = 5;
// 400 kHz Fast Mode with typical rise/fall times
const TARGET_FREQ: u32 = 400_000;
const RISE_NS: u32 = 100;
const FALL_NS: u32 = 10;

#[entry]
fn main() -> ! {
    utilities::logger::init();

    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.vos0().freeze();

    let rcc = dp.RCC.constrain();
    let ccdr = rcc.sys_ck(250.MHz()).freeze(pwrcfg, &dp.SBS);

    let ker_clk = ccdr.clocks.sys_ck().raw();
    let dwt = cp.DWT.constrain(cp.DCB, &ccdr.clocks);

    info!("I2C timing benchmark: N={}, target={} Hz, ker_clk={} Hz", N, TARGET_FREQ, ker_clk);

    let mut samples = [0u32; N];
    let mut result = (0u8, 0u8, 0u8, 0u8, 0u8);

    for sample in samples.iter_mut() {
        let cd = dwt.measure(|| {
            result = calc_timing_bench(ker_clk, TARGET_FREQ, RISE_NS, FALL_NS);
        });
        *sample = cd.as_ticks() as u32;
    }

    // prevent optimizer from eliding the calls
    info!("result (presc,scll,sclh,sdadel,scldel): ({},{},{},{},{})",
        result.0, result.1, result.2, result.3, result.4);

    // stats
    let min = samples.iter().copied().min().unwrap_or(0);
    let max = samples.iter().copied().max().unwrap_or(0);
    let sum: u64 = samples.iter().map(|&x| x as u64).sum();
    let mean = sum / N as u64;

    let variance: f32 = samples.iter()
        .map(|&x| {
            let diff = x as f32 - mean as f32;
            diff * diff
        })
        .sum::<f32>() / N as f32;
    let stddev = {
        // Newton-Raphson sqrt (no libm needed in no_std)
        if variance == 0.0 {
            0.0f32
        } else {
            let mut x = variance;
            for _ in 0..20 { x = 0.5 * (x + variance / x); }
            x
        }
    };

    let clk_mhz = ker_clk / 1_000_000;
    info!("samples   : {}", N);
    info!("min       : {} ticks  ({} ns)", min, min / clk_mhz);
    info!("max       : {} ticks  ({} ns)", max, max / clk_mhz);
    info!("mean      : {} ticks  ({} ns)", mean, mean / clk_mhz as u64);
    info!("stddev    : {:.1} ticks", stddev);

    loop {}
}
