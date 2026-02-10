//! Dreamcast controller state representation.
//!
//! Holds the parsed state from a `Get Condition` (`0x09`) response.

use crate::maple::host::functions;

/// Represents the state of a standard Dreamcast controller.
#[derive(Debug, Clone, Copy, Default)]
pub struct ControllerState {
    /// Digital button states (active LOW in protocol, but we store as active HIGH here).
    pub buttons: ButtonState,

    /// Left trigger analog value (0-255, 0 = released, 255 = fully pressed).
    pub trigger_l: u8,

    /// Right trigger analog value (0-255, 0 = released, 255 = fully pressed).
    pub trigger_r: u8,

    /// Analog stick X axis (0-255, 128 = center, 0 = left, 255 = right).
    pub stick_x: u8,

    /// Analog stick Y axis (0-255, 128 = center, 0 = up, 255 = down).
    pub stick_y: u8,
}

/// Digital button states from a Dreamcast controller.
/// Note: In the Maple protocol, buttons are active LOW (0 = pressed).
/// We invert them here so true = pressed for easier use.
#[allow(clippy::struct_excessive_bools)]
#[allow(dead_code)] // All buttons defined for completeness; not all used yet
#[derive(Debug, Clone, Copy, Default)]
pub struct ButtonState {
    pub c: bool,
    pub b: bool,
    pub a: bool,
    pub start: bool,
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
    pub z: bool,
    pub y: bool,
    pub x: bool,
    pub d: bool, // Second D button (rare)
                 // Bits 12-15 are typically unused on standard controllers
}

impl ButtonState {
    /// Parse button state from the first data word of a `Get Condition` response.
    /// The button bits are in the upper 16 bits of the first payload word.
    /// Buttons are active LOW in the protocol, so we invert.
    #[must_use]
    pub fn from_raw(raw: u16) -> Self {
        Self {
            c: (raw & (1 << 0)) == 0,
            b: (raw & (1 << 1)) == 0,
            a: (raw & (1 << 2)) == 0,
            start: (raw & (1 << 3)) == 0,
            dpad_up: (raw & (1 << 4)) == 0,
            dpad_down: (raw & (1 << 5)) == 0,
            dpad_left: (raw & (1 << 6)) == 0,
            dpad_right: (raw & (1 << 7)) == 0,
            z: (raw & (1 << 8)) == 0,
            y: (raw & (1 << 9)) == 0,
            x: (raw & (1 << 10)) == 0,
            d: (raw & (1 << 11)) == 0,
        }
    }

    /// Returns true if any button is currently pressed.
    #[must_use]
    #[allow(dead_code)] // Utility method for future use
    pub fn any_pressed(&self) -> bool {
        self.c
            || self.b
            || self.a
            || self.start
            || self.dpad_up
            || self.dpad_down
            || self.dpad_left
            || self.dpad_right
            || self.z
            || self.y
            || self.x
            || self.d
    }

    /// Convert button state back to raw `u16` format for BLE transmission.
    /// Bit is set (1) when button is pressed (opposite of Maple protocol).
    #[must_use]
    #[allow(dead_code)] // Utility method for future use
    pub fn to_raw(self) -> u16 {
        let mut raw: u16 = 0;
        if self.c {
            raw |= 1 << 0;
        }
        if self.b {
            raw |= 1 << 1;
        }
        if self.a {
            raw |= 1 << 2;
        }
        if self.start {
            raw |= 1 << 3;
        }
        if self.dpad_up {
            raw |= 1 << 4;
        }
        if self.dpad_down {
            raw |= 1 << 5;
        }
        if self.dpad_left {
            raw |= 1 << 6;
        }
        if self.dpad_right {
            raw |= 1 << 7;
        }
        if self.z {
            raw |= 1 << 8;
        }
        if self.y {
            raw |= 1 << 9;
        }
        if self.x {
            raw |= 1 << 10;
        }
        if self.d {
            raw |= 1 << 11;
        }
        raw
    }
}

/// Dreamcast analog stick center value (0-255 range).
const DC_STICK_CENTER: u8 = 128;

/// Xbox BLE stick center value (unsigned 16-bit, 0-65535 range).
const XBOX_STICK_CENTER: u16 = 32768;

/// Scale factor to convert Dreamcast stick (0-255) to Xbox stick (0-65535).
/// Maps 0→0, 128→32896≈32768, 255→65535.
const STICK_SCALE_FACTOR: u16 = 257;

/// Maximum Xbox trigger value (10-bit).
const XBOX_TRIGGER_MAX: u32 = 1023;

/// Maximum Dreamcast trigger value (8-bit).
const DC_TRIGGER_MAX: u32 = 255;

impl ControllerState {
    /// Convert to Xbox One S BLE HID gamepad report.
    ///
    /// Mapping:
    /// - Dreamcast A/B/X/Y -> Xbox Buttons 1-4
    /// - Dreamcast Start -> Xbox Button 8 (Menu)
    /// - Dreamcast D-pad -> Hat switch (1-8, 0=neutral)
    /// - Dreamcast analog stick -> Left stick (uint16, 0-65535, center=32768)
    /// - Dreamcast L/R triggers -> Brake/Accelerator (10-bit, 0-1023)
    #[must_use]
    #[allow(clippy::items_after_statements)]
    pub fn to_gamepad_report(self) -> crate::ble::hid::GamepadReport {
        use crate::ble::hid::{buttons, hat, GamepadReport};

        let mut btns: u16 = 0;
        if self.buttons.a {
            btns |= buttons::A;
        }
        if self.buttons.b {
            btns |= buttons::B;
        }
        if self.buttons.x {
            btns |= buttons::X;
        }
        if self.buttons.y {
            btns |= buttons::Y;
        }
        if self.buttons.start {
            btns |= buttons::START;
        }

        // D-pad → Hat switch (Xbox convention: 1-8, 0=neutral)
        let hat_value = match (
            self.buttons.dpad_up,
            self.buttons.dpad_down,
            self.buttons.dpad_left,
            self.buttons.dpad_right,
        ) {
            (true, false, false, false) => hat::NORTH,
            (true, false, false, true) => hat::NORTH_EAST,
            (false, false, false, true) => hat::EAST,
            (false, true, false, true) => hat::SOUTH_EAST,
            (false, true, false, false) => hat::SOUTH,
            (false, true, true, false) => hat::SOUTH_WEST,
            (false, false, true, false) => hat::WEST,
            (true, false, true, false) => hat::NORTH_WEST,
            _ => hat::NEUTRAL,
        };

        // Convert Dreamcast stick (u8, 0-255, center=128) to Xbox (u16, 0-65535, center=32768)
        // Scale: multiply by 257 (maps 0→0, 128→32896≈32768, 255→65535)
        // Apply deadzone around center
        const DEADZONE: u16 = 10;
        let raw_x = self.stick_y; // Dreamcast Y -> HID X
        let raw_y = self.stick_x; // Dreamcast X -> HID Y
        let left_x: u16 = if (i16::from(raw_x) - i16::from(DC_STICK_CENTER)).unsigned_abs() < DEADZONE {
            XBOX_STICK_CENTER
        } else {
            u16::from(raw_x) * STICK_SCALE_FACTOR
        };
        let left_y: u16 = if (i16::from(raw_y) - i16::from(DC_STICK_CENTER)).unsigned_abs() < DEADZONE {
            XBOX_STICK_CENTER
        } else {
            u16::from(raw_y) * STICK_SCALE_FACTOR
        };

        // Convert triggers: 0-255 -> 0-1023 (10-bit)
        #[allow(clippy::cast_possible_truncation)]
        let left_trigger = (u32::from(self.trigger_l) * XBOX_TRIGGER_MAX / DC_TRIGGER_MAX) as u16;
        #[allow(clippy::cast_possible_truncation)]
        let right_trigger = (u32::from(self.trigger_r) * XBOX_TRIGGER_MAX / DC_TRIGGER_MAX) as u16;

        GamepadReport {
            left_x,
            left_y,
            left_trigger,
            right_trigger,
            hat: hat_value,
            buttons: btns,
        }
    }

    /// Parse controller state from a `Get Condition` response payload.
    ///
    /// Expected payload format (from command `0x09` response):
    /// - Word 0: Function type (should be `0x0000_0001` for controller)
    /// - Word 1: Buttons (upper 16 bits) + unused (lower 16 bits)
    /// - Word 2: Triggers (R in upper byte, L in next) + Stick X, Y
    ///
    /// Returns `None` if payload is too short or function type is wrong.
    #[must_use]
    pub fn from_payload(payload: &[u32]) -> Option<Self> {
        if payload.len() < 3 {
            return None;
        }

        // Word 0: Function type - must be standard controller
        let func_type = payload[0];
        if func_type != functions::CONTROLLER {
            return None; // Not a standard controller
        }

        // Word 1 format (bytes on wire): [trig_L, trig_R, btn_low, btn_high]
        // Assembled: trig_L | (trig_R << 8) | (btn_low << 16) | (btn_high << 24)
        // Raw values: 0x00 = released, 0xFF = fully pressed (no inversion needed)
        let word1 = payload[1];
        #[allow(clippy::cast_possible_truncation)]
        let trigger_l = (word1 & 0xFF) as u8;
        #[allow(clippy::cast_possible_truncation)]
        let trigger_r = ((word1 >> 8) & 0xFF) as u8;

        // Buttons in upper 16 bits, bytes swapped
        #[allow(clippy::cast_possible_truncation)]
        let buttons_word = ((word1 >> 16) & 0xFFFF) as u16;
        let buttons_raw = buttons_word.swap_bytes();
        let buttons = ButtonState::from_raw(buttons_raw);

        // Word 2: Analog sticks
        // Format: [unused, unused, stick_x, stick_y] (main stick in upper 16 bits)
        // Bytes 0-1 are for secondary stick (stays 0x80 on standard controller)
        let analog_word = payload[2];
        #[allow(clippy::cast_possible_truncation)]
        let stick_x = ((analog_word >> 16) & 0xFF) as u8;
        #[allow(clippy::cast_possible_truncation)]
        let stick_y = ((analog_word >> 24) & 0xFF) as u8;

        Some(Self {
            buttons,
            trigger_l,
            trigger_r,
            stick_x,
            stick_y,
        })
    }

    /// Check if the stick is roughly centered (within deadzone).
    #[must_use]
    #[allow(dead_code)] // Utility method for future use
    pub fn stick_centered(&self, deadzone: u8) -> bool {
        let dx = (i16::from(self.stick_x) - i16::from(DC_STICK_CENTER)).unsigned_abs();
        let dy = (i16::from(self.stick_y) - i16::from(DC_STICK_CENTER)).unsigned_abs();
        dx <= u16::from(deadzone) && dy <= u16::from(deadzone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_parse_none_pressed() {
        // All bits high = no buttons pressed (active low)
        let buttons = ButtonState::from_raw(0xFFFF);
        assert!(!buttons.any_pressed());
    }

    #[test]
    fn test_button_parse_a_pressed() {
        // Bit 2 low = A pressed
        let buttons = ButtonState::from_raw(0xFFFB);
        assert!(buttons.a);
        assert!(!buttons.b);
        assert!(!buttons.start);
    }

    #[test]
    fn test_controller_state_parse() {
        // Word 1: [trig_L_raw, trig_R_raw, btn_low, btn_high]
        // Triggers inverted: 0xFF = released (becomes 0), 0x00 = pressed (becomes 255)
        // For trigger_l = 200 (after invert): raw = 255 - 200 = 55 = 0x37
        // For trigger_r = 100 (after invert): raw = 255 - 100 = 155 = 0x9B
        // Buttons: A pressed = bit 2 low. After swap_bytes on upper 16: need 0xFFFB
        // Upper 16 bits = btn_low | (btn_high << 8), so bytes [0xFB, 0xFF]
        // Word 1 = 0x37 | (0x9B << 8) | (0xFB << 16) | (0xFF << 24) = 0xFFFB9B37
        //
        // Word 2: [stick_x, stick_y, ...]
        // For stick_x = 64, stick_y = 200: Word 2 = 64 | (200 << 8) | ... = 0x????C840
        let payload = [
            0x0000_0001, // Function type: controller
            0xFFFB_9B37, // trig_L_raw=0x37, trig_R_raw=0x9B, buttons=A pressed
            0x8080_C840, // stick_x=64, stick_y=200, unused
        ];

        let state = ControllerState::from_payload(&payload).unwrap();
        assert!(state.buttons.a);
        assert_eq!(state.trigger_l, 200);
        assert_eq!(state.trigger_r, 100);
        assert_eq!(state.stick_x, 64);
        assert_eq!(state.stick_y, 200);
    }
}
