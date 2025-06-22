//! Configuration constants for the Dreamcast BLE Adapter.

/// Enable debug messages over UART0.
/// Warning: enabling debug messages drastically degrades communication performance.
pub const SHOW_DEBUG_MESSAGES: bool = false;

/// Enable USB CDC (serial) interface to directly control the Maple bus.
pub const USB_CDC_ENABLED: bool = true;

/// Enable USB MSC (mass storage) interface to read/write VMU files.
pub const USB_MSC_ENABLED: bool = true;

/// CPU clock frequency in kHz (133000 kHz = 133 MHz is recommended).
pub const CPU_FREQ_KHZ: u32 = 133_000;

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

/// IO direction pin for each player (-1 to disable).
pub const P1_DIR_PIN: i8 = 6;
pub const P2_DIR_PIN: i8 = 7;
pub const P3_DIR_PIN: i8 = 26;
pub const P4_DIR_PIN: i8 = 27;

/// True if DIR pin is HIGH for output and LOW for input; false if opposite.
pub const DIR_OUT_HIGH: bool = true;

/// Start pin
