//! nRF52840-specific board configuration and pin definitions.
//!
//! This module provides type-safe GPIO pin definitions for the Maple Bus
//! interface on the nRF52840-DK development board.

use nrf52840_dk_bsp::hal::gpio::{p0, Input, Output, Pin, PullUp, PushPull, Floating};

/// Maple Bus pin configuration.
///
/// The Maple Bus uses two bidirectional lines (SDCKA and SDCKB).
/// On the nRF52840-DK, we use P0.05 and P0.06 which are on the main header.
///
/// # Hardware Connection
/// ```text
/// nRF52840-DK          Dreamcast Controller
/// -----------          --------------------
/// P0.05 (SDCKA) <----> Pin 1 (D0/SDCKA)
/// P0.06 (SDCKB) <----> Pin 5 (D1/SDCKB)
/// GND           <----> Pin 3 (GND)
/// 3.3V          <----> Pin 2 (VCC) [if powering controller from DK]
/// ```
///
/// # Pin Notes
/// - Maple Bus is 5V tolerant but 3.3V signaling works
/// - Lines are active-low, directly between MCU pins (no level shifter)
/// - Both lines are bidirectional; configure as input to receive, output to transmit

/// Type alias for SDCKA pin configured as input (receiving).
pub type SdckaInput = Pin<Input<PullUp>>;

/// Type alias for SDCKB pin configured as input (receiving).
pub type SdckbInput = Pin<Input<PullUp>>;

/// Type alias for SDCKA pin configured as output (transmitting).
pub type SdckaOutput = Pin<Output<PushPull>>;

/// Type alias for SDCKB pin configured as output (transmitting).
pub type SdckbOutput = Pin<Output<PushPull>>;

/// Maple Bus pin indices on P0.
pub mod pins {
    /// SDCKA pin number on P0.
    pub const SDCKA: usize = 5;
    /// SDCKB pin number on P0.
    pub const SDCKB: usize = 6;
}

/// Holds the GPIO pins for Maple Bus communication.
///
/// The pins need to switch between input and output modes during communication,
/// so this struct holds them in a state that can be reconfigured.
pub struct MaplePins<SDCKA, SDCKB> {
    pub sdcka: SDCKA,
    pub sdckb: SDCKB,
}

impl MaplePins<p0::P0_05<Input<Floating>>, p0::P0_06<Input<Floating>>> {
    /// Create new MaplePins from freshly taken P0 pins.
    ///
    /// The pins start in floating input mode and should be configured
    /// appropriately before use.
    pub fn new(
        sdcka: p0::P0_05<Input<Floating>>,
        sdckb: p0::P0_06<Input<Floating>>,
    ) -> Self {
        Self { sdcka, sdckb }
    }
}
