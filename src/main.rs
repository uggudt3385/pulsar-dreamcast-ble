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

#[entry]
fn main() -> ! {
    // Board Support Package
    // A crate that knows board layout, layer on top of
    let mut nrf52 = Board::take().unwrap();

    let mut timer = Timer::new(nrf52.TIMER0);

    loop {
        nrf52.leds.led_4.enable();
        delay(&mut timer, 250_000); // 250ms
        nrf52.leds.led_4.disable();
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

// use cortex_m::asm::nop;
// use cortex_m_rt::entry;
// use embedded_hal::digital::{OutputPin, PinState};
// use hal::pac;
// use nrf52840_hal::{self as hal, gpio::Level};

// use panic_halt as _;
// use rtt_target::{rprint, rtt_init_print};

// // Hardware Abstraction Layer (HAL)
// // Built on top of PAC, allows work at higher level
// let p = pac::Peripherals::take().unwrap();
// let port0 = hal::gpio::p0::Parts::new(p.P0);
// let mut led3 = port0.p0_15.into_push_pull_output(Level::Low);
// let mut is_on: bool = false;
// loop {
//     let _ = led3.set_state(PinState::from(is_on));
//     for _ in 0..100_000 {
//         nop();
//     }
//     is_on = !is_on;
// }

// // Peripheral Access Crate (PAC)
// use nrf52840_pac::Peripherals;
// let p = Peripherals::take().unwrap();
// p.P0.pin_cnf[14].write(|w| w.dir().output());
// let mut is_on: bool = false;
// loop {
//     p.P0.out.write(|w| w.pin14().bit(is_on));
//     for _ in 0..500_000 {
//         nop();
//     }
//     is_on = !is_on;
// }

// Blinky Unsafe Rust
// const GPIO0_PINCNF13_LED1_ADDR: *mut u32 = 0x5000_0734 as *mut u32;
// const DIR_OUTPUT_POS: u32 = 0;
// const PINCNF_DRIVE_LED: u32 = 1 << DIR_OUTPUT_POS;
// unsafe {
//     write_volatile(GPIO0_PINCNF13_LED1_ADDR, PINCNF_DRIVE_LED);
// }
// const GPIO0_OUT_ADDR: *mut u32 = 0x5000_0504 as *mut u32;
// const GPIO0_OUT_LED1_POS: u32 = 13;
// let mut is_on = false;
// loop {
//     unsafe {
//         write_volatile(GPIO0_OUT_ADDR, (is_on as u32) << GPIO0_OUT_LED1_POS);
//     }
//     for _ in 0..250_000 {
//         nop();
//     }
//     is_on = !is_on;
// }
