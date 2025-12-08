#![no_std]
#![no_main]

use cortex_m::asm::nop;
// nrf Dependencies
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

// Maple Dependencies
mod maple;
use crate::maple::{MockMapleBus, state_machine::MapleController, traits::MapleBusTrait};
// use defmt_rtt as _;
// use panic_probe as _;

const MAX_DEVICES: usize = 1;

const MAPLE_HOST_ADDRESSES: u8 = 0x00;

fn monotonic() -> u64 {
    0 // Replace with timer later
}

// defmt::timestamp!("{=u64}", monotonic());

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Starting mock Maple bus cycle..");

    // Confirm LED flashing works
    let mut board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);

    // // Test LED blinking first
    for i in 0..2 {
        rprintln!("Blink cycle: {}", i);
        board.leds.led_1.enable(); // LED ON
        delay(&mut timer, 250_000); // 250ms
        board.leds.led_1.disable(); // LED OFF
        board.leds.led_2.enable(); // LED ON
        delay(&mut timer, 250_000); // 250ms
        board.leds.led_2.disable(); // LED OFF
        board.leds.led_3.enable(); // LED ON
        delay(&mut timer, 250_000); // 250ms
        board.leds.led_3.disable(); // LED OFF
        board.leds.led_4.enable(); // LED ON
        delay(&mut timer, 250_000); // 250ms
        board.leds.led_4.disable(); // LED OFF
        delay(&mut timer, 250_000);
    }
    board.leds.led_1.enable(); // LED ON

    // rprintln!("Blink complete. Proceeding to mock Maple bus logic.");

    // // Create a mock bus
    // let mut bus = MockMapleBus::new();

    // // Create a controller with a reference to the bus
    // let mut controller = MapleController::new(&mut bus);

    // // Simulate a timestamp
    // let now_us = 1000;

    // // Step the controller (this will call next_state, write to bus, etc.)
    // controller.step(now_us);

    // // Let the mock bus process its internal state (if applicable)
    // let status = bus.process_events(now_us);

    // rprintln!("Bus status after processing: {:?}", status);

    // loop {
    //     cortex_m::asm::wfi();
    // }
    let mut timeout_timer = Timer::new(board.TIMER1);

    loop {
        // Poll button state
        let pressed = board.buttons.button_2.is_pressed();
        rprintln!("Button 2 pressed: {}", pressed);

        if pressed {
            board.leds.led_2.enable(); // Turn on when button is pressed
        } else {
            board.leds.led_2.disable(); // Off otherwise
        }

        //     timeout_timer.start(15_000_000u32); // microseconds
        //     loop {
        //         // Rapid blink LED4
        //         board.leds.led_4.enable();
        //         delay(&mut timer, 100_000); // 100ms
        //         board.leds.led_4.disable();
        //         delay(&mut timer, 100_000); // 100ms

        //         // Exit after 15s
        //         if block!(timeout_timer.wait()).is_ok() {
        //             rprintln!("Timer expired, exiting blink loop.");
        //             break;
        //         }
        //     }
        // }
        nop();
        // // Sleep to reduce CPU usage between checks
        delay(&mut timer, 50_000); // 50ms debounce delay
    }
}

fn delay<T>(timer: &mut Timer<T>, cycles: u32)
where
    T: timer::Instance,
{
    timer.start(cycles);
    let _ = block!(timer.wait());
}
