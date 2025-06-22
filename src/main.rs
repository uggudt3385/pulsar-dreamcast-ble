#![no_std]
#![no_main]

use cortex_m_rt::entry;
use nb::block;

use panic_halt as _;

use nrf52840_dk_bsp::{
    Board,
    hal::{
        prelude::*,
        timer::{self, Timer},
    },
};
use rtt_target::{rprintln, rtt_init_print};

const MAX_DEVICES: usize = 1;

const MAPLE_HOST_ADDRESSES: u8 = 0x00;

#[entry]
fn main() -> ! {
    // Board Support Package
    // A crate that knows board layout, layer on top of
    let mut nrf52 = Board::take().unwrap();

    let mut timer = Timer::new(nrf52.TIMER0);

    rtt_init_print!();
    rprintln!("Hello, Dreamcast BLE Adapter!");

    loop {
        rprintln!("looping...");
        nrf52.leds.led_2.enable();
        delay(&mut timer, 350_000); // 250ms
        nrf52.leds.led_2.disable();
        delay(&mut timer, 1_000_000);
    }
}

fn delay<T>(timer: &mut Timer<T>, cycles: u32)
where
    T: timer::Instance,
{
    timer.start(cycles);
    let _ = block!(timer.wait());
}
