//! Board support for the Seeed XIAO nRF52840.
//!
//! Pin assignments:
//! - SDCKA: P0.05 (D5), SDCKB: P0.04 (D4)
//! - RGB LED: R=P0.26, G=P0.30, B=P0.06 (all active LOW, internal)
//! - Sync button: P0.29 (D3, wired to VMU MODE button)
//! - Wake button: P0.02 (D0, wired to VMU SLEEP button, GPIO SENSE wake)
//! - Boost SHDN: P0.28 (D2, HIGH=on, LOW=shutdown)
//! - Battery ADC: P0.31 (internal, via P0.14 enable — future)

use embassy_nrf::gpio::{Flex, Input, Level, Output, OutputDrive, Pin, Pull};
use embassy_nrf::Peri;
use embassy_time::{Duration, Timer};

/// SDCKA bit position in P0 GPIO register.
pub const PIN_A_BIT: u32 = 5; // P0.05 (D5)

/// SDCKB bit position in P0 GPIO register.
pub const PIN_B_BIT: u32 = 4; // P0.04 (D4)

/// Whether this board supports System Off sleep mode.
#[allow(dead_code)] // Part of board abstraction API
pub const SUPPORTS_SLEEP: bool = true;

/// Status LEDs for the main task (R and G channels of the RGB LED).
///
/// The blue channel is owned by the sync button task.
///
/// Color mapping:
/// - Searching: Red solid
/// - Controller found / BLE connected: Green solid
pub struct StatusLeds {
    led_r: Output<'static>,
    led_g: Output<'static>,
}

impl StatusLeds {
    /// Blink green for startup indication.
    pub async fn startup_blink(&mut self) {
        for _ in 0..3 {
            self.led_g.set_low();
            Timer::after(Duration::from_millis(100)).await;
            self.led_g.set_high();
            Timer::after(Duration::from_millis(100)).await;
        }
    }

    /// Indicate controller search in progress (red solid).
    pub fn show_searching(&mut self) {
        self.led_g.set_high();
        self.led_r.set_low();
    }

    /// Indicate controller found (green solid).
    pub fn show_controller_found(&mut self) {
        self.led_r.set_high();
        self.led_g.set_low();
    }

    /// Turn on TX activity indicator (no-op on XIAO to avoid flicker).
    #[allow(clippy::unused_self)] // Must match DK API
    pub fn tx_activity_on(&mut self) {}

    /// Turn off TX activity indicator (no-op on XIAO).
    #[allow(clippy::unused_self)] // Must match DK API
    pub fn tx_activity_off(&mut self) {}
}

/// Initialize all board-specific pins.
///
/// Returns `(sdcka, sdckb, sync_button, sync_led, status_leds)`.
///
/// The blue LED channel is passed out as `sync_led` for the sync button task.
/// The boost converter (SHDN pin) is enabled at init and stored in a static
/// for later shutdown during sleep.
#[allow(clippy::similar_names, clippy::type_complexity)]
pub fn init_pins(
    sdcka_pin: Peri<'static, impl Pin>,
    sdckb_pin: Peri<'static, impl Pin>,
    led_r_pin: Peri<'static, impl Pin>,
    led_g_pin: Peri<'static, impl Pin>,
    led_b_pin: Peri<'static, impl Pin>,
    button_pin: Peri<'static, impl Pin>,
    boost_pin: Peri<'static, impl Pin>,
) -> (
    Flex<'static>,
    Flex<'static>,
    Input<'static>,
    Output<'static>,
    StatusLeds,
) {
    let sdcka = Flex::new(sdcka_pin);
    let sdckb = Flex::new(sdckb_pin);
    let sync_button = Input::new(button_pin, Pull::Up);

    let led_r = Output::new(led_r_pin, Level::High, OutputDrive::Standard);
    let led_g = Output::new(led_g_pin, Level::High, OutputDrive::Standard);
    let sync_led = Output::new(led_b_pin, Level::High, OutputDrive::Standard);

    // Enable 5V boost converter on startup
    let boost = Output::new(boost_pin, Level::High, OutputDrive::Standard);
    // Store in static for sleep/shutdown access
    // SAFETY: Written once here, read only from main task during sleep entry
    unsafe {
        BOOST_CONTROL = Some(boost);
    }

    let status = StatusLeds { led_r, led_g };

    (sdcka, sdckb, sync_button, sync_led, status)
}

/// P0 GPIO base address for register access.
const P0_BASE: u32 = 0x5000_0000;
/// Offset to `PIN_CNF` registers within GPIO peripheral.
const PIN_CNF_OFFSET: u32 = 0x700;
/// Wake pin number (P0.02 / D0).
const WAKE_PIN_NUM: u32 = 2;

/// Static storage for the boost converter control pin.
/// Used during System Off entry to disable 5V output.
static mut BOOST_CONTROL: Option<Output<'static>> = None;

/// Disable the 5V boost converter before entering System Off.
///
/// # Safety
/// Must only be called from the main task context, after `init_pins`.
pub unsafe fn disable_boost() {
    if let Some(ref mut boost) = BOOST_CONTROL {
        boost.set_low();
    }
}

/// Enter System Off mode (deep sleep, ~5µA draw).
///
/// Configures the wake pin (D0/P0.02) with GPIO SENSE for wake-on-press,
/// disables the 5V boost converter, then enters System Off via `SoftDevice`.
///
/// On wake, the device performs a full reset (boots fresh).
///
/// # Safety
/// This function does not return. The `SoftDevice` must be initialized.
pub unsafe fn enter_system_off(wake_pin: Peri<'static, impl Pin>) -> ! {
    use rtt_target::rprintln;

    rprintln!("SLEEP: Disabling boost converter");
    disable_boost();

    // Configure wake pin with SENSE LOW (button press = LOW)
    // The pin needs to be configured as input with pull-up and SENSE enabled
    let _wake = Input::new(wake_pin, Pull::Up);

    // Configure SENSE on the pin via raw register access
    // P0.02 = pin 2, PIN_CNF[2] needs SENSE = Low (3)
    let cnf_addr = (P0_BASE + PIN_CNF_OFFSET + WAKE_PIN_NUM * 4) as *mut u32;
    let cnf = core::ptr::read_volatile(cnf_addr);
    // Set SENSE field (bits 17:16) to 3 (Low)
    let cnf = (cnf & !(0x3 << 16)) | (0x3 << 16);
    core::ptr::write_volatile(cnf_addr, cnf);

    rprintln!("SLEEP: Entering System Off");

    // Enter System Off via SoftDevice
    nrf_softdevice::raw::sd_power_system_off();

    // Should never reach here
    loop {
        cortex_m::asm::wfi();
    }
}
