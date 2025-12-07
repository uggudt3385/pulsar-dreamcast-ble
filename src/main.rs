#![no_std]
#![no_main]

mod maple;
use crate::maple::{
    MaplePacket, MockMapleBus, state_machine::MapleController, traits::MapleBusTrait,
};
use defmt_rtt as _;
use heapless::Vec;
use panic_probe as _;

use cortex_m_rt::entry;
// use nb::block;

// use panic_halt as _;

// use nrf52840_dk_bsp::{
//     Board,
//     hal::{
//         prelude::*,
//         timer::{self, Timer},
//     },
// };
// use rtt_target::{rprintln, rtt_init_print};

const MAX_DEVICES: usize = 1;

const MAPLE_HOST_ADDRESSES: u8 = 0x00;

fn monotonic() -> u64 {
    0 // Replace with timer later
}

defmt::timestamp!("{=u64}", monotonic());

#[entry]
fn main() -> ! {
    defmt::info!("Starting mock Maple bus cycle..");

    // Create a mock bus
    let mut bus = MockMapleBus::new();

    // Create a controller with a reference to the bus
    let mut controller = MapleController::new(&mut bus);

    // Simulate a timestamp
    let now_us = 1000;

    // Step the controller (this will call next_state, write to bus, etc.)
    controller.step(now_us);

    // Let the mock bus process its internal state (if applicable)
    let status = bus.process_events(now_us);

    defmt::info!("Bus status after processing: {:?}", status);

    loop {
        cortex_m::asm::wfi();
    }
}
