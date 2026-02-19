//! Board support for the Seeed XIAO nRF52840.
//!
//! Pin assignments:
//! - SDCKA: P0.05 (D5), SDCKB: P0.03 (D1)
//! - RGB LED: R=P0.26, G=P0.30, B=P0.06 (all active LOW, internal)
//! - Sync button: P1.15 (D10, wired to VMU MODE button)
//! - Wake button: P0.02 (D0, wired to VMU SLEEP button, GPIO SENSE wake)
//! - Boost SHDN: P0.28 (D2, HIGH=on, LOW=shutdown)
//! - Battery ADC: P0.31 (internal, via P0.14 enable — future)

use embassy_nrf::gpio::{Flex, Input, Level, Output, OutputDrive, Pin, Pull};
use embassy_nrf::saadc::{self, Saadc};
use embassy_nrf::Peri;
use embassy_time::{Duration, Timer};

/// SDCKA bit position in P0 GPIO register.
pub const PIN_A_BIT: u32 = 5; // P0.05 (D5)

/// SDCKB bit position in P0 GPIO register.
pub const PIN_B_BIT: u32 = 3; // P0.03 (D1)

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

/// Initialized board pins, ready for use by the main task.
pub struct BoardPins {
    pub sdcka: Flex<'static>,
    pub sdckb: Flex<'static>,
    pub sync_button: Input<'static>,
    pub sync_led: Output<'static>,
    pub status: StatusLeds,
    pub charge_stat: Input<'static>,
}

/// Initialize all board-specific pins.
///
/// The blue LED channel is passed out as `sync_led` for the sync button task.
/// The boost converter (SHDN pin) is enabled at init and stored in a static
/// for later shutdown during sleep. The charge current is set to 100mA (P0.13 LOW)
/// and the BQ25101 STAT pin (P0.17) is returned for charge status monitoring.
#[allow(clippy::similar_names)]
pub fn init_pins(
    sdcka_pin: Peri<'static, impl Pin>,
    sdckb_pin: Peri<'static, impl Pin>,
    led_r_pin: Peri<'static, impl Pin>,
    led_g_pin: Peri<'static, impl Pin>,
    led_b_pin: Peri<'static, impl Pin>,
    button_pin: Peri<'static, impl Pin>,
    boost_pin: Peri<'static, impl Pin>,
    charge_pin: Peri<'static, impl Pin>,
    charge_stat_pin: Peri<'static, impl Pin>,
) -> BoardPins {
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

    // Set charge current to 100mA (P0.13 LOW on XIAO BQ25101)
    // Pin config persists after drop — just need to set it once.
    let _charge = Output::new(charge_pin, Level::Low, OutputDrive::Standard);

    // BQ25101 STAT pin: LOW = charging, HIGH = not charging / full
    let charge_stat = Input::new(charge_stat_pin, Pull::Up);

    let status = StatusLeds { led_r, led_g };

    BoardPins {
        sdcka,
        sdckb,
        sync_button,
        sync_led,
        status,
        charge_stat,
    }
}

/// P1 GPIO base address for register access.
const P1_BASE: u32 = 0x5000_0300;
/// Offset to `PIN_CNF` registers within GPIO peripheral.
const PIN_CNF_OFFSET: u32 = 0x700;
/// Wake pin number (P1.15 / D10 — sync button doubles as wake).
const WAKE_PIN_NUM: u32 = 15;

/// Static storage for the boost converter control pin.
/// Used during System Off entry to disable 5V output.
///
/// # Safety
/// Written once in `init_pins()`, read only from `disable_boost()` during
/// sleep entry. Both run on the main task of a single-core Cortex-M4 —
/// no concurrent access possible.
static mut BOOST_CONTROL: Option<Output<'static>> = None;

/// Disable the 5V boost converter before entering System Off.
///
/// # Safety
/// Must only be called from the main task context, after `init_pins`.
pub unsafe fn disable_boost() {
    // SAFETY: See BOOST_CONTROL declaration — single writer, single reader, no concurrency.
    if let Some(ref mut boost) = BOOST_CONTROL {
        boost.set_low();
    }
}

/// Battery voltage reader using SAADC on P0.31 (AIN7).
///
/// The XIAO has a 1:2 voltage divider on P0.31, gated by P0.14 (HIGH=enable).
/// With the internal 0.6V reference and 1/6 gain, the SAADC input range is 0-3.6V,
/// which maps to 0-7.2V battery voltage after the 2:1 divider.
pub struct BatteryReader<'d> {
    saadc: Saadc<'d, 1>,
    enable: Output<'d>,
}

impl<'d> BatteryReader<'d> {
    /// Create a new battery reader.
    ///
    /// `enable_pin` is P0.14 (drives the voltage divider gate).
    /// `adc_pin` is P0.31 (AIN7, battery voltage through divider).
    /// `saadc_peri` is the SAADC peripheral.
    pub fn new(
        enable_pin: Peri<'d, impl Pin>,
        adc_pin: impl saadc::Input + 'd,
        saadc_peri: Peri<'d, embassy_nrf::peripherals::SAADC>,
        irq: impl embassy_nrf::interrupt::typelevel::Binding<
                embassy_nrf::interrupt::typelevel::SAADC,
                saadc::InterruptHandler,
            > + 'd,
    ) -> Self {
        let enable = Output::new(enable_pin, Level::Low, OutputDrive::Standard);
        let channel = saadc::ChannelConfig::single_ended(adc_pin);
        let saadc = Saadc::new(saadc_peri, irq, saadc::Config::default(), [channel]);
        Self { saadc, enable }
    }

    /// Read battery voltage and return `(millivolts, percentage)`.
    ///
    /// Enables the voltage divider, takes a sample, disables divider.
    ///
    /// 12-bit SAADC with 0.6V internal ref, 1/6 gain gives 0-3.6V range.
    /// Battery voltage = ADC voltage * 2 (1:2 divider).
    /// Battery range: 3.0V (empty) to 4.2V (full).
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub async fn read(&mut self) -> (u32, u8) {
        self.enable.set_high();
        // Brief settling time for the voltage divider
        Timer::after(Duration::from_micros(100)).await;

        let mut buf = [0i16; 1];
        self.saadc.sample(&mut buf).await;

        self.enable.set_low();

        let raw = buf[0].max(0) as u32;
        // 12-bit resolution (0-4095), internal ref 0.6V, gain 1/6 → full scale 3.6V
        // Voltage at ADC pin = raw * 3.6 / 4095
        // Battery voltage = ADC voltage * 2 (voltage divider)
        // Fixed-point: v_bat_mv = raw * 7200 / 4095
        let v_bat_mv = raw * 7200 / 4095;

        // LiPo: 3000mV = 0%, 4200mV = 100%
        let percent = if v_bat_mv <= 3000 {
            0
        } else if v_bat_mv >= 4200 {
            100
        } else {
            ((v_bat_mv - 3000) * 100 / 1200) as u8
        };

        (v_bat_mv, percent)
    }
}

/// Enter System Off mode (deep sleep, ~5µA draw).
///
/// Configures the sync button (D10/P1.15) with GPIO SENSE for wake-on-press,
/// disables the 5V boost converter, then enters System Off via `SoftDevice`.
/// The sync button pin is already configured as input with pull-up by the
/// sync button task — we just add SENSE to it.
///
/// On wake, the device performs a full reset (boots fresh).
///
/// # Safety
/// This function does not return. The `SoftDevice` must be initialized.
pub unsafe fn enter_system_off() -> ! {
    use rtt_target::rprintln;

    // Turn off all LEDs (active low: HIGH = off)
    // P0 OUTSET register: set P0.26 (R), P0.30 (G), P0.06 (B)
    const P0_OUTSET: *mut u32 = 0x5000_0508 as *mut u32;
    core::ptr::write_volatile(P0_OUTSET, (1 << 26) | (1 << 30) | (1 << 6));

    rprintln!("SLEEP: Disabling boost converter");
    disable_boost();

    // Configure wake pin: input with pull-up + SENSE LOW
    // P1.15 = PIN_CNF[15] on P1
    let cnf_addr = (P1_BASE + PIN_CNF_OFFSET + WAKE_PIN_NUM * 4) as *mut u32;
    let cnf_before = core::ptr::read_volatile(cnf_addr);
    // DIR=Input(0), INPUT=Connected(0), PULL=Pullup(3<<2), SENSE=Low(3<<16)
    let cnf = (cnf_before & !(0x3 << 16) & !(0x3 << 2)) | (0x3 << 16) | (0x3 << 2);
    core::ptr::write_volatile(cnf_addr, cnf);
    let cnf_after = core::ptr::read_volatile(cnf_addr);
    rprintln!(
        "SLEEP: P1.{} CNF 0x{:08X} -> 0x{:08X}",
        WAKE_PIN_NUM,
        cnf_before,
        cnf_after
    );

    // Enter System Off via SoftDevice
    nrf_softdevice::raw::sd_power_system_off();

    // Should never reach here
    loop {
        cortex_m::asm::wfi();
    }
}
