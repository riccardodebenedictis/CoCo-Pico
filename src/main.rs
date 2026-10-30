#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_rp::init(Default::default());
    info!("Embassy RP2040 example started");

    let mut counter: u32 = 0;
    loop {
        info!("Heartbeat tick #{}", counter);
        counter = counter.wrapping_add(1);
        Timer::after(Duration::from_secs(1)).await;
    }
}
