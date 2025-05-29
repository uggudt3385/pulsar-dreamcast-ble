#![no_std]
#![no_main]

use cortex_m::asm::nop;
use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprint, rtt_init_print};
#[entry]
fn main() -> ! {
    let mut _x: usize = 0;
    loop {
        _x += 1;
        for _ in 0..100_000 {
            nop();
        }
    }
}
