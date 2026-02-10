//! HID over GATT (HOG) implementation for gamepad.
//!
//! Implements Xbox One S BLE HID format (Model 1708, PID `0x02E0`).

#![allow(clippy::redundant_else)] // Macro-generated code
#![allow(clippy::missing_errors_doc)] // Internal API
#![allow(clippy::trivially_copy_pass_by_ref)] // Macro-generated _set methods
#![allow(clippy::unnecessary_semicolon)] // Macro-generated code
#![allow(dead_code)] // Macro-generated event enum fields

use heapless::Vec;
use nrf_softdevice::ble::gatt_server::{NotifyValueError, SetValueError};
use nrf_softdevice::ble::Connection;

/// HID Report Descriptor for Xbox One S BLE controller format.
///
/// Uses xpadneo-patched usage convention for broad HID parser compatibility:
///   - Left stick:  X (0x30) / Y (0x31)    — Generic Desktop
///   - Right stick:  Rx (0x33) / Ry (0x34)  — Generic Desktop
///   - Triggers:    Z (0x32) / Rz (0x35)    — Generic Desktop
///
/// Report ID 0x01 - Main input (16 bytes):
///   Bytes 0-1:   Left Stick X   (uint16, 0-65535, center=32768)
///   Bytes 2-3:   Left Stick Y   (uint16, 0-65535, center=32768)
///   Bytes 4-5:   Right Stick X  (uint16, 0-65535, center=32768)
///   Bytes 6-7:   Right Stick Y  (uint16, 0-65535, center=32768)
///   Bytes 8-9:   Left Trigger   (10-bit 0-1023 + 6-bit padding)
///   Bytes 10-11: Right Trigger  (10-bit 0-1023 + 6-bit padding)
///   Byte 12:     Hat Switch     (4-bit 1-8, 0=null + 4-bit padding)
///   Bytes 13-14: Buttons 1-15   (15 bits + 1-bit padding)
///   Byte 15:     AC Back        (1 bit + 7-bit padding)
///
/// Report ID 0x02 - Xbox/Guide button (1 byte):
///   Byte 0: AC Home (1 bit + 7-bit padding)
///
/// Report ID 0x03 - Force feedback output (9 bytes, host→device)
#[rustfmt::skip]
pub const HID_REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x01,        // Usage Page (Generic Desktop)
    0x09, 0x05,        // Usage (Gamepad)
    0xA1, 0x01,        // Collection (Application)

    // === Report ID 0x01: Main Gamepad Input ===
    0x85, 0x01,        //   Report ID (1)

    // Left Stick (Physical collection, unsigned 16-bit)
    0x09, 0x01,        //   Usage (Pointer)
    0xA1, 0x00,        //   Collection (Physical)
    0x09, 0x30,        //     Usage (X)
    0x09, 0x31,        //     Usage (Y)
    0x15, 0x00,        //     Logical Minimum (0)
    0x27, 0xFF, 0xFF, 0x00, 0x00, //  Logical Maximum (65535)
    0x95, 0x02,        //     Report Count (2)
    0x75, 0x10,        //     Report Size (16)
    0x81, 0x02,        //     Input (Data, Variable, Absolute)
    0xC0,              //   End Collection

    // Right Stick (Physical collection, unsigned 16-bit)
    // Uses Rx/Ry (standard convention, matches xpadneo-patched Xbox descriptor)
    0x09, 0x01,        //   Usage (Pointer)
    0xA1, 0x00,        //   Collection (Physical)
    0x09, 0x33,        //     Usage (Rx)
    0x09, 0x34,        //     Usage (Ry)
    0x15, 0x00,        //     Logical Minimum (0)
    0x27, 0xFF, 0xFF, 0x00, 0x00, //  Logical Maximum (65535)
    0x95, 0x02,        //     Report Count (2)
    0x75, 0x10,        //     Report Size (16)
    0x81, 0x02,        //     Input (Data, Variable, Absolute)
    0xC0,              //   End Collection

    // Left Trigger (Generic Desktop Z, 10-bit + 6 padding)
    // Uses Z/Rz (standard convention, matches xpadneo-patched Xbox descriptor)
    0x05, 0x01,        //   Usage Page (Generic Desktop)
    0x09, 0x32,        //   Usage (Z)
    0x15, 0x00,        //   Logical Minimum (0)
    0x26, 0xFF, 0x03,  //   Logical Maximum (1023)
    0x95, 0x01,        //   Report Count (1)
    0x75, 0x0A,        //   Report Size (10)
    0x81, 0x02,        //   Input (Data, Variable, Absolute)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x00,        //   Logical Maximum (0)
    0x75, 0x06,        //   Report Size (6)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x03,        //   Input (Constant) - padding

    // Right Trigger (Generic Desktop Rz, 10-bit + 6 padding)
    0x09, 0x35,        //   Usage (Rz)
    0x15, 0x00,        //   Logical Minimum (0)
    0x26, 0xFF, 0x03,  //   Logical Maximum (1023)
    0x95, 0x01,        //   Report Count (1)
    0x75, 0x0A,        //   Report Size (10)
    0x81, 0x02,        //   Input (Data, Variable, Absolute)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x00,        //   Logical Maximum (0)
    0x75, 0x06,        //   Report Size (6)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x03,        //   Input (Constant) - padding

    // Hat Switch / D-pad (4-bit value + 4-bit padding)
    0x05, 0x01,        //   Usage Page (Generic Desktop)
    0x09, 0x39,        //   Usage (Hat Switch)
    0x15, 0x01,        //   Logical Minimum (1)
    0x25, 0x08,        //   Logical Maximum (8)
    0x35, 0x00,        //   Physical Minimum (0)
    0x46, 0x3B, 0x01,  //   Physical Maximum (315)
    0x66, 0x14, 0x00,  //   Unit (Degrees)
    0x75, 0x04,        //   Report Size (4)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x42,        //   Input (Data, Variable, Absolute, Null State)
    0x75, 0x04,        //   Report Size (4)
    0x95, 0x01,        //   Report Count (1)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x00,        //   Logical Maximum (0)
    0x35, 0x00,        //   Physical Minimum (0)
    0x45, 0x00,        //   Physical Maximum (0)
    0x65, 0x00,        //   Unit (None)
    0x81, 0x03,        //   Input (Constant) - padding

    // Buttons 1-15
    0x05, 0x09,        //   Usage Page (Button)
    0x19, 0x01,        //   Usage Minimum (Button 1)
    0x29, 0x0F,        //   Usage Maximum (Button 15)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x01,        //   Logical Maximum (1)
    0x75, 0x01,        //   Report Size (1)
    0x95, 0x0F,        //   Report Count (15)
    0x81, 0x02,        //   Input (Data, Variable, Absolute)
    // 1-bit padding
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x00,        //   Logical Maximum (0)
    0x75, 0x01,        //   Report Size (1)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x03,        //   Input (Constant) - padding

    // AC Back (Consumer Control, 1-bit + 7-bit padding)
    0x05, 0x0C,        //   Usage Page (Consumer)
    0x0A, 0x24, 0x02,  //   Usage (AC Back)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x01,        //   Logical Maximum (1)
    0x95, 0x01,        //   Report Count (1)
    0x75, 0x01,        //   Report Size (1)
    0x81, 0x02,        //   Input (Data, Variable, Absolute)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x00,        //   Logical Maximum (0)
    0x75, 0x07,        //   Report Size (7)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x03,        //   Input (Constant) - padding

    // === Report ID 0x02: Xbox/Guide Button ===
    0x05, 0x0C,        //   Usage Page (Consumer)
    0x09, 0x01,        //   Usage (Consumer Control)
    0x85, 0x02,        //   Report ID (2)
    0xA1, 0x01,        //   Collection (Application)
    0x05, 0x0C,        //     Usage Page (Consumer)
    0x0A, 0x23, 0x02,  //     Usage (AC Home)
    0x15, 0x00,        //     Logical Minimum (0)
    0x25, 0x01,        //     Logical Maximum (1)
    0x95, 0x01,        //     Report Count (1)
    0x75, 0x01,        //     Report Size (1)
    0x81, 0x02,        //     Input (Data, Variable, Absolute)
    0x15, 0x00,        //     Logical Minimum (0)
    0x25, 0x00,        //     Logical Maximum (0)
    0x75, 0x07,        //     Report Size (7)
    0x95, 0x01,        //     Report Count (1)
    0x81, 0x03,        //     Input (Constant) - padding
    0xC0,              //   End Collection

    // === Report ID 0x03: Rumble Output ===
    0x05, 0x0F,        //   Usage Page (Physical Interface Device)
    0x09, 0x21,        //   Usage (Set Effect Report)
    0x85, 0x03,        //   Report ID (3)
    0xA1, 0x02,        //   Collection (Logical)
    0x09, 0x97,        //     Usage (DC Enable Actuators)
    0x15, 0x00,        //     Logical Minimum (0)
    0x25, 0x01,        //     Logical Maximum (1)
    0x75, 0x04,        //     Report Size (4)
    0x95, 0x01,        //     Report Count (1)
    0x91, 0x02,        //     Output (Data, Variable, Absolute)
    0x15, 0x00,        //     Logical Minimum (0)
    0x25, 0x00,        //     Logical Maximum (0)
    0x75, 0x04,        //     Report Size (4)
    0x95, 0x01,        //     Report Count (1)
    0x91, 0x03,        //     Output (Constant) - padding
    0x09, 0x70,        //     Usage (Magnitude)
    0x15, 0x00,        //     Logical Minimum (0)
    0x25, 0x64,        //     Logical Maximum (100)
    0x75, 0x08,        //     Report Size (8)
    0x95, 0x04,        //     Report Count (4)
    0x91, 0x02,        //     Output (Data, Variable, Absolute)
    0x09, 0x50,        //     Usage (Duration)
    0x66, 0x01, 0x10,  //     Unit (SI Linear: Time)
    0x55, 0x0E,        //     Unit Exponent (-2)
    0x15, 0x00,        //     Logical Minimum (0)
    0x26, 0xFF, 0x00,  //     Logical Maximum (255)
    0x75, 0x08,        //     Report Size (8)
    0x95, 0x01,        //     Report Count (1)
    0x91, 0x02,        //     Output (Data, Variable, Absolute)
    0x09, 0xA7,        //     Usage (Start Delay)
    0x15, 0x00,        //     Logical Minimum (0)
    0x26, 0xFF, 0x00,  //     Logical Maximum (255)
    0x75, 0x08,        //     Report Size (8)
    0x95, 0x01,        //     Report Count (1)
    0x91, 0x02,        //     Output (Data, Variable, Absolute)
    0x65, 0x00,        //     Unit (None)
    0x55, 0x00,        //     Unit Exponent (0)
    0x09, 0x7C,        //     Usage (Loop Count)
    0x15, 0x00,        //     Logical Minimum (0)
    0x26, 0xFF, 0x00,  //     Logical Maximum (255)
    0x75, 0x08,        //     Report Size (8)
    0x95, 0x01,        //     Report Count (1)
    0x91, 0x02,        //     Output (Data, Variable, Absolute)
    0xC0,              //   End Collection

    // === Report ID 0x04: Battery ===
    0x05, 0x06,        //   Usage Page (Generic Device Controls)
    0x09, 0x20,        //   Usage (Battery Strength)
    0x85, 0x04,        //   Report ID (4)
    0x15, 0x00,        //   Logical Minimum (0)
    0x26, 0xFF, 0x00,  //   Logical Maximum (255)
    0x75, 0x08,        //   Report Size (8)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x02,        //   Input (Data, Variable, Absolute)

    0xC0,              // End Collection
];

/// HID Information characteristic value.
/// bcdHID: 1.11, bCountryCode: 0, Flags: `RemoteWake` | `NormallyConnectable`
pub const HID_INFO: [u8; 4] = [0x11, 0x01, 0x00, 0x03];

/// 10-bit trigger mask.
const TRIGGER_10BIT_MASK: u16 = 0x03FF;

/// Right stick center as little-endian bytes (32768 = 0x8000).
const RIGHT_STICK_CENTER_LE: [u8; 2] = [0x00, 0x80];

/// Protocol Mode: Report Protocol (1) vs Boot Protocol (0)
pub const PROTOCOL_MODE_REPORT: u8 = 1;

/// Xbox One S BLE gamepad report (Report ID 0x01, 16 bytes data).
///
/// NOTE: Report ID is NOT included in the byte array — the Report Reference
/// descriptor on the characteristic identifies this as Report ID 1.
///
/// Byte layout matches real Xbox One S (Model 1708) exactly:
///   Bytes 0-1:   Left Stick X   (uint16 LE, 0-65535, center=32768)
///   Bytes 2-3:   Left Stick Y   (uint16 LE, 0-65535, center=32768)
///   Bytes 4-5:   Right Stick X  (uint16 LE, 0-65535, center=32768)
///   Bytes 6-7:   Right Stick Y  (uint16 LE, 0-65535, center=32768)
///   Bytes 8-9:   Left Trigger   (10-bit LE in low bits, 6 padding in high bits)
///   Bytes 10-11: Right Trigger  (10-bit LE in low bits, 6 padding in high bits)
///   Byte 12:     Hat Switch     (4-bit in low nibble, 4 padding in high nibble)
///   Bytes 13-14: Buttons 1-15   (15 bits, 1-bit padding)
///   Byte 15:     AC Back        (1 bit, 7-bit padding)
#[derive(Clone, Copy)]
pub struct GamepadReport {
    /// Left stick X (0=left, 32768=center, 65535=right)
    pub left_x: u16,
    /// Left stick Y (0=top, 32768=center, 65535=bottom)
    pub left_y: u16,
    /// Left trigger (0=released, 1023=fully pressed)
    pub left_trigger: u16,
    /// Right trigger (0=released, 1023=fully pressed)
    pub right_trigger: u16,
    /// Hat switch / D-pad (0=neutral, 1-8=directions)
    pub hat: u8,
    /// Button bitmask (bits 0-14 = buttons 1-15)
    pub buttons: u16,
}

/// Xbox BLE stick center value (unsigned 16-bit).
const STICK_CENTER: u16 = 32768;

impl Default for GamepadReport {
    fn default() -> Self {
        Self {
            left_x: STICK_CENTER,
            left_y: STICK_CENTER,
            left_trigger: 0,
            right_trigger: 0,
            hat: hat::NEUTRAL,
            buttons: 0,
        }
    }
}

/// Hat switch values (Xbox One convention: 1-8, 0=neutral/null).
pub mod hat {
    pub const NEUTRAL: u8 = 0;
    pub const NORTH: u8 = 1;
    pub const NORTH_EAST: u8 = 2;
    pub const EAST: u8 = 3;
    pub const SOUTH_EAST: u8 = 4;
    pub const SOUTH: u8 = 5;
    pub const SOUTH_WEST: u8 = 6;
    pub const WEST: u8 = 7;
    pub const NORTH_WEST: u8 = 8;
}

impl GamepadReport {
    /// Create a new report with neutral/centered values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert to 16-byte array for BLE transmission.
    ///
    /// Trigger packing: 10 data bits in low bits of `u16`, 6 zero padding in high bits.
    /// Byte 8 = `trigger[7:0]`, Byte 9 = `000000 | trigger[9:8]`
    #[must_use]
    pub fn to_bytes(self) -> [u8; 16] {
        let lx = self.left_x.to_le_bytes();
        let ly = self.left_y.to_le_bytes();
        // Triggers: mask to 10 bits, stored as LE u16 (padding is in high 6 bits)
        let lt = (self.left_trigger & TRIGGER_10BIT_MASK).to_le_bytes();
        let rt = (self.right_trigger & TRIGGER_10BIT_MASK).to_le_bytes();

        #[allow(clippy::cast_possible_truncation)]
        [
            // Left Stick X (bytes 0-1, uint16 LE)
            lx[0],
            lx[1],
            // Left Stick Y (bytes 2-3, uint16 LE)
            ly[0],
            ly[1],
            // Right Stick X (bytes 4-5) - Dreamcast has no right stick, center=32768
            RIGHT_STICK_CENTER_LE[0],
            RIGHT_STICK_CENTER_LE[1],
            // Right Stick Y (bytes 6-7)
            RIGHT_STICK_CENTER_LE[0],
            RIGHT_STICK_CENTER_LE[1],
            // Left Trigger (bytes 8-9, 10-bit + 6 padding)
            lt[0],
            lt[1],
            // Right Trigger (bytes 10-11, 10-bit + 6 padding)
            rt[0],
            rt[1],
            // Hat Switch (byte 12, low nibble = value, high nibble = padding)
            self.hat & 0x0F,
            // Buttons 1-8 (byte 13)
            (self.buttons & 0xFF) as u8,
            // Buttons 9-15 + 1-bit padding (byte 14)
            ((self.buttons >> 8) & 0x7F) as u8,
            // AC Back (byte 15, bit 0) - unused, always 0
            0x00,
        ]
    }
}

/// Button bit positions in the 15-bit button field.
/// Xbox One S layout (matches HID Button Usage 1-15):
///   Bit 0  = Button 1  = A
///   Bit 1  = Button 2  = B
///   Bit 2  = Button 3  = X
///   Bit 3  = Button 4  = Y
///   Bit 4  = Button 5  = LB (Left Bumper)
///   Bit 5  = Button 6  = RB (Right Bumper)
///   Bit 6  = Button 7  = Back/View
///   Bit 7  = Button 8  = Menu/Start
///   Bit 8  = Button 9  = Left Stick Click (L3)
///   Bit 9  = Button 10 = Right Stick Click (R3)
///   Bits 10-14 = reserved
#[allow(dead_code)] // Reference constants for Xbox button layout
pub mod buttons {
    pub const A: u16 = 1 << 0;
    pub const B: u16 = 1 << 1;
    pub const X: u16 = 1 << 2;
    pub const Y: u16 = 1 << 3;
    pub const LB: u16 = 1 << 4;
    pub const RB: u16 = 1 << 5;
    pub const BACK: u16 = 1 << 6;
    pub const START: u16 = 1 << 7;
    pub const L3: u16 = 1 << 8;
    pub const R3: u16 = 1 << 9;
}

// GATT Service definitions using nrf-softdevice macros

/// HID Service (UUID 0x1812)
/// Security: `JustWorks` (encrypted, unauthenticated) - required by HOGP spec
#[allow(dead_code)] // Macro-generated fields
#[nrf_softdevice::gatt_service(uuid = "1812")]
pub struct HidService {
    /// HID Information (UUID 0x2A4A) - Read only
    /// Value: [bcdHID_lo, bcdHID_hi, bCountryCode, flags]
    #[characteristic(uuid = "2A4A", read, security = "JustWorks")]
    pub hid_info: [u8; 4],

    /// Report Map (UUID 0x2A4B) - Read only, contains HID descriptor
    #[characteristic(uuid = "2A4B", read, security = "JustWorks")]
    pub report_map: Vec<u8, 512>,

    /// HID Report - Input (UUID 0x2A4D), Report ID 1
    /// Main gamepad state (16 bytes)
    #[characteristic(
        uuid = "2A4D",
        read,
        notify,
        security = "JustWorks",
        descriptor(uuid = "2908", security = "JustWorks", value = "[0x01, 0x01]")
    )]
    pub report: [u8; 16],

    /// HID Control Point (UUID 0x2A4C) - Write without response
    #[characteristic(uuid = "2A4C", write_without_response, security = "JustWorks")]
    pub control_point: u8,

    /// Protocol Mode (UUID 0x2A4E) - Read, Write Without Response
    #[characteristic(uuid = "2A4E", read, write_without_response, security = "JustWorks")]
    pub protocol_mode: u8,
}

/// Device Information Service (UUID 0x180A)
#[allow(dead_code)] // Macro-generated fields
#[nrf_softdevice::gatt_service(uuid = "180A")]
pub struct DeviceInfoService {
    /// Manufacturer Name (UUID 0x2A29)
    #[characteristic(uuid = "2A29", read)]
    pub manufacturer: Vec<u8, 32>,

    /// Model Number (UUID 0x2A24)
    #[characteristic(uuid = "2A24", read)]
    pub model_number: Vec<u8, 32>,

    /// PnP ID (UUID 0x2A50) - Vendor ID, Product ID, Version
    #[characteristic(uuid = "2A50", read)]
    pub pnp_id: [u8; 7],
}

/// Battery Service (UUID 0x180F)
#[allow(dead_code)] // Macro-generated fields
#[nrf_softdevice::gatt_service(uuid = "180F")]
pub struct BatteryService {
    /// Battery Level (UUID 0x2A19) - 0-100%
    #[characteristic(uuid = "2A19", read, notify)]
    pub battery_level: u8,
}

/// Combined GATT server with all services.
#[allow(dead_code)] // Macro-generated fields
#[nrf_softdevice::gatt_server]
pub struct GamepadServer {
    pub hid: HidService,
    pub device_info: DeviceInfoService,
    pub battery: BatteryService,
}

impl GamepadServer {
    /// Initialize the server with default values.
    pub fn init(&self) -> Result<(), SetValueError> {
        self.hid.hid_info_set(&HID_INFO)?;

        let mut report_map: Vec<u8, 512> = Vec::new();
        let _ = report_map.extend_from_slice(HID_REPORT_DESCRIPTOR).ok();
        self.hid.report_map_set(&report_map)?;

        self.hid.protocol_mode_set(&PROTOCOL_MODE_REPORT)?;

        // Initial report: sticks centered (32768), everything else zero
        let initial_report = GamepadReport::new();
        self.hid.report_set(&initial_report.to_bytes())?;

        // Device Information - match Xbox One S
        let mut manufacturer: Vec<u8, 32> = Vec::new();
        let _ = manufacturer.extend_from_slice(b"Microsoft").ok();
        self.device_info.manufacturer_set(&manufacturer)?;

        let mut model: Vec<u8, 32> = Vec::new();
        let _ = model.extend_from_slice(b"Xbox Wireless Controller").ok();
        self.device_info.model_number_set(&model)?;

        // PnP ID: Xbox One S Controller over BLE
        let pnp_id: [u8; 7] = [
            0x02, // Vendor ID Source (USB-IF)
            0x5E, 0x04, // Vendor ID: 0x045E (Microsoft)
            0xE0, 0x02, // Product ID: 0x02E0 (Xbox One S BLE)
            0x00, 0x01, // Version 1.0
        ];
        self.device_info.pnp_id_set(&pnp_id)?;

        self.battery.battery_level_set(&100)?;

        Ok(())
    }

    /// Send a gamepad report notification.
    pub fn send_report(
        &self,
        conn: &Connection,
        report: &GamepadReport,
    ) -> Result<(), NotifyValueError> {
        self.hid.report_notify(conn, &report.to_bytes())
    }
}
