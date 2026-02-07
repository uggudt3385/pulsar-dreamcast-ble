#![no_std]
#![no_main]

use cortex_m_rt::entry;
use nb::block;
use panic_halt as _;

use nrf52840_dk_bsp::hal::{
    gpio::{Level, p0::Parts as P0Parts},
    pac,
    prelude::*,
    timer::{self, Timer},
};

use rtt_target::{rprintln, rtt_init_print};

mod maple;

use crate::maple::host::MapleResult;
use crate::maple::{ControllerState, MapleBusGpio, MapleHost};

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Dreamcast Controller Adapter Starting...");

    let periph = pac::Peripherals::take().unwrap();
    let p0 = P0Parts::new(periph.P0);
    let mut timer = Timer::new(periph.TIMER0);

    // LEDs (active low on DK)
    let mut led1 = p0.p0_13.into_push_pull_output(Level::High).degrade();
    let mut led2 = p0.p0_14.into_push_pull_output(Level::High).degrade();
    let mut led3 = p0.p0_15.into_push_pull_output(Level::High).degrade();
    let mut led4 = p0.p0_16.into_push_pull_output(Level::High).degrade();

    // Startup blink
    for _ in 0..3 {
        let _ = led1.set_low();
        delay(&mut timer, 100_000);
        let _ = led1.set_high();
        delay(&mut timer, 100_000);
    }

    // Check initial bus state
    let test_a = p0.p0_05.into_pullup_input();
    let test_b = p0.p0_06.into_pullup_input();
    delay(&mut timer, 10_000);
    let a = test_a.is_high().unwrap_or(false);
    let b = test_b.is_high().unwrap_or(false);
    rprintln!("Bus state: A={} B={}", a as u8, b as u8);

    // Set up Maple Bus
    let sdcka = test_a.into_push_pull_output(Level::High).degrade();
    let sdckb = test_b.into_push_pull_output(Level::Low).degrade();
    let mut bus = MapleBusGpio::new(sdcka, sdckb);
    let host = MapleHost::new();

    // Detect controller
    rprintln!("Detecting controller...");
    let _ = led2.set_low();

    let (new_bus, result) = host.request_device_info(bus);
    bus = new_bus;

    let controller_detected = match &result {
        MapleResult::Ok(info) => {
            rprintln!("Controller found! Functions: 0x{:08X}", info.functions);
            let _ = led2.set_high();
            let _ = led3.set_low();
            true
        }
        _ => {
            rprintln!("No controller detected");
            let _ = led2.set_high();
            let _ = led4.set_low();
            false
        }
    };

    if !controller_detected {
        rprintln!("Halting - no controller");
        loop {
            cortex_m::asm::wfi();
        }
    }

    // Controller polling loop
    rprintln!("");
    rprintln!("=== Polling Controller Input ===");
    rprintln!("Press buttons on the Dreamcast controller!");
    rprintln!("");

    let mut last_state: Option<ControllerState> = None;
    let mut poll_count: u32 = 0;

    loop {
        let (new_bus, result) = host.get_condition(bus);
        bus = new_bus;

        match result {
            MapleResult::Ok(state) => {
                // LED1 on when any button pressed
                if state.buttons.any_pressed() {
                    let _ = led1.set_low();
                } else {
                    let _ = led1.set_high();
                }

                // Only print when state changes
                let changed = match &last_state {
                    None => true,
                    Some(prev) => state_changed(prev, &state),
                };

                if changed {
                    print_state(&state);
                    last_state = Some(state);
                }
            }
            MapleResult::Timeout => {
                if poll_count % 100 == 0 {
                    rprintln!("Poll timeout");
                }
            }
            MapleResult::CrcError => {
                rprintln!("CRC error");
            }
            MapleResult::UnexpectedResponse(cmd) => {
                rprintln!("Unexpected: 0x{:02X}", cmd);
            }
        }

        poll_count = poll_count.wrapping_add(1);
        delay(&mut timer, 16_000); // ~60Hz polling
    }
}

fn state_changed(prev: &ControllerState, curr: &ControllerState) -> bool {
    // Check buttons
    if prev.buttons.a != curr.buttons.a
        || prev.buttons.b != curr.buttons.b
        || prev.buttons.x != curr.buttons.x
        || prev.buttons.y != curr.buttons.y
        || prev.buttons.start != curr.buttons.start
        || prev.buttons.dpad_up != curr.buttons.dpad_up
        || prev.buttons.dpad_down != curr.buttons.dpad_down
        || prev.buttons.dpad_left != curr.buttons.dpad_left
        || prev.buttons.dpad_right != curr.buttons.dpad_right
    {
        return true;
    }

    // Check triggers (with deadzone)
    if (prev.trigger_l as i16 - curr.trigger_l as i16).abs() > 10
        || (prev.trigger_r as i16 - curr.trigger_r as i16).abs() > 10
    {
        return true;
    }

    // Check stick (with deadzone)
    if (prev.stick_x as i16 - curr.stick_x as i16).abs() > 15
        || (prev.stick_y as i16 - curr.stick_y as i16).abs() > 15
    {
        return true;
    }

    false
}

fn print_state(state: &ControllerState) {
    let b = &state.buttons;

    // Build button string
    let mut btns: heapless::String<32> = heapless::String::new();
    if b.a {
        let _ = btns.push_str("A ");
    }
    if b.b {
        let _ = btns.push_str("B ");
    }
    if b.x {
        let _ = btns.push_str("X ");
    }
    if b.y {
        let _ = btns.push_str("Y ");
    }
    if b.start {
        let _ = btns.push_str("ST ");
    }
    if b.dpad_up {
        let _ = btns.push_str("U ");
    }
    if b.dpad_down {
        let _ = btns.push_str("D ");
    }
    if b.dpad_left {
        let _ = btns.push_str("L ");
    }
    if b.dpad_right {
        let _ = btns.push_str("R ");
    }

    if btns.is_empty() {
        rprintln!(
            "Stick({},{}) Trig({},{})",
            state.stick_x,
            state.stick_y,
            state.trigger_l,
            state.trigger_r
        );
    } else {
        rprintln!(
            "[{}] Stick({},{}) Trig({},{})",
            btns.trim_end(),
            state.stick_x,
            state.stick_y,
            state.trigger_l,
            state.trigger_r
        );
    }
}

fn delay<T>(timer: &mut Timer<T>, cycles: u32)
where
    T: timer::Instance,
{
    timer.start(cycles);
    let _ = block!(timer.wait());
}
