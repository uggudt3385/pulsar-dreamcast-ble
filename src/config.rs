//! Configuration constants for the Dreamcast BLE Adapter.
//! Target: nRF52840 (ARM Cortex-M4F, 64 MHz)

/// Enable debug messages over RTT.
/// Warning: enabling debug messages may affect timing-critical operations.
pub const SHOW_DEBUG_MESSAGES: bool = false;

/// Enable USB CDC (serial) interface to directly control the Maple bus.
pub const USB_CDC_ENABLED: bool = false; // Not yet implemented

/// Enable USB MSC (mass storage) interface to read/write VMU files.
pub const USB_MSC_ENABLED: bool = false; // Not yet implemented

/// CPU clock frequency in kHz (64000 kHz = 64 MHz for nRF52840).
pub const CPU_FREQ_KHZ: u32 = 64_000;

/// CPU clock frequency in Hz.
pub const CPU_FREQ_HZ: u32 = CPU_FREQ_KHZ * 1_000;

/// Nanoseconds per CPU cycle (15.625 ns at 64 MHz).
pub const NS_PER_CYCLE: u32 = 1_000_000 / CPU_FREQ_KHZ; // ~15ns

/// Minimum time to check for an open line before taking control of it (microseconds).
/// Set to 0 to disable this check.
pub const MAPLE_OPEN_LINE_CHECK_TIME_US: u32 = 10;

/// Time per bit in nanoseconds (value should be divisible by 3).
/// 480 ns achieves ~2 Mbps like the Dreamcast.
pub const MAPLE_NS_PER_BIT: u32 = 480;

/// Additional percentage added to expected write completion duration for timeout.
pub const MAPLE_WRITE_TIMEOUT_EXTRA_PERCENT: u8 = 20;

/// Estimated nanoseconds before a peripheral responds (for scheduling only).
pub const MAPLE_RESPONSE_DELAY_NS: u32 = 50;

/// Max time to wait for the beginning of a response when one is expected (microseconds).
pub const MAPLE_RESPONSE_TIMEOUT_US: u32 = 1000;

/// Estimated nanoseconds per bit to receive data (for scheduling only).
pub const MAPLE_RESPONSE_NS_PER_BIT: u32 = 1750;

/// Max time (microseconds) allowed between received words before read is canceled.
/// 300 us accommodates for ~180 us gaps from some Dreamcast controllers.
pub const MAPLE_INTER_WORD_READ_TIMEOUT_US: u32 = 300;

/// Maple Bus GPIO pin assignments for nRF52840-DK.
/// Using P0 pins that are exposed on the DK board headers.
/// SDCKA and SDCKB are the two bidirectional data/clock lines.
pub const MAPLE_SDCKA_PIN: u8 = 5;  // P0.05
pub const MAPLE_SDCKB_PIN: u8 = 6;  // P0.06

/// Optional direction control pin for level shifter/buffer (-1 to disable).
/// Set high for output, low for input (if DIR_OUT_HIGH is true).
pub const MAPLE_DIR_PIN: i8 = -1;  // Disabled - direct 3.3V connection

/// True if DIR pin is HIGH for output and LOW for input; false if opposite.
pub const DIR_OUT_HIGH: bool = true;

/// Maple Bus polling interval in microseconds.
/// 16ms = 60Hz polling rate (matches typical game refresh).
pub const MAPLE_POLL_INTERVAL_US: u32 = 16_000;
