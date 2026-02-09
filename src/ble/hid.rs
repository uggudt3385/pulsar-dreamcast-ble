//! HID over GATT (HOG) implementation for gamepad.
//!
//! Implements standard BLE HID service with Xbox-compatible gamepad layout.

use heapless::Vec;
use nrf_softdevice::ble::gatt_server::{NotifyValueError, SetValueError};
use nrf_softdevice::ble::Connection;

/// HID Report Descriptor for gamepad (Xbox-compatible layout).
///
/// Layout (15 bytes total):
/// - 12 buttons + 4 padding (2 bytes)
/// - Hat switch / D-pad (1 byte: 4 bits value + 4 bits padding)
/// - Left stick X/Y: 16-bit signed each (4 bytes)
/// - Right stick X/Y: 16-bit signed each (4 bytes)
/// - Left trigger: 16-bit unsigned (2 bytes)
/// - Right trigger: 16-bit unsigned (2 bytes)
#[rustfmt::skip]
pub const HID_REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x01,        // Usage Page (Generic Desktop)
    0x09, 0x05,        // Usage (Gamepad)
    0xA1, 0x01,        // Collection (Application)

    // Report ID 1
    0x85, 0x01,        //   Report ID (1)

    // 12 Buttons + 4 padding bits (2 bytes)
    0x05, 0x09,        //   Usage Page (Button)
    0x19, 0x01,        //   Usage Minimum (Button 1)
    0x29, 0x0C,        //   Usage Maximum (Button 12)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x01,        //   Logical Maximum (1)
    0x75, 0x01,        //   Report Size (1)
    0x95, 0x0C,        //   Report Count (12)
    0x81, 0x02,        //   Input (Data, Variable, Absolute)
    // 4 bits padding to complete the 2 bytes
    0x75, 0x01,        //   Report Size (1)
    0x95, 0x04,        //   Report Count (4)
    0x81, 0x03,        //   Input (Constant) - padding

    // Hat Switch / D-pad (4 bits + 4 bits padding = 1 byte)
    0x05, 0x01,        //   Usage Page (Generic Desktop)
    0x09, 0x39,        //   Usage (Hat Switch)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x07,        //   Logical Maximum (7)
    0x35, 0x00,        //   Physical Minimum (0)
    0x46, 0x3B, 0x01,  //   Physical Maximum (315 degrees)
    0x65, 0x14,        //   Unit (Eng Rot: Degree)
    0x75, 0x04,        //   Report Size (4)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x42,        //   Input (Data, Variable, Absolute, Null State)
    // 4 bits padding
    0x75, 0x04,        //   Report Size (4)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x03,        //   Input (Constant) - padding

    // Left stick - 16-bit signed axes (-32768 to 32767)
    0x05, 0x01,        //   Usage Page (Generic Desktop)
    0x09, 0x30,        //   Usage (X) - Left stick X
    0x09, 0x31,        //   Usage (Y) - Left stick Y
    0x16, 0x00, 0x80,  //   Logical Minimum (-32768)
    0x26, 0xFF, 0x7F,  //   Logical Maximum (32767)
    0x75, 0x10,        //   Report Size (16)
    0x95, 0x02,        //   Report Count (2)
    0x81, 0x02,        //   Input (Data, Variable, Absolute)

    // Right stick - 16-bit signed axes (-32768 to 32767)
    0x09, 0x33,        //   Usage (Rx) - Right stick X
    0x09, 0x34,        //   Usage (Ry) - Right stick Y
    0x16, 0x00, 0x80,  //   Logical Minimum (-32768)
    0x26, 0xFF, 0x7F,  //   Logical Maximum (32767)
    0x75, 0x10,        //   Report Size (16)
    0x95, 0x02,        //   Report Count (2)
    0x81, 0x02,        //   Input (Data, Variable, Absolute)

    // Triggers - 10-bit unsigned (0 to 1023), stored in 16-bit for alignment
    0x09, 0x32,        //   Usage (Z) - Left trigger
    0x09, 0x35,        //   Usage (Rz) - Right trigger
    0x15, 0x00,        //   Logical Minimum (0)
    0x26, 0xFF, 0x03,  //   Logical Maximum (1023)
    0x75, 0x10,        //   Report Size (16)
    0x95, 0x02,        //   Report Count (2)
    0x81, 0x02,        //   Input (Data, Variable, Absolute)

    0xC0,              // End Collection
];

/// HID Information characteristic value.
/// bcdHID: 1.11, bCountryCode: 0, Flags: RemoteWake | NormallyConnectable
pub const HID_INFO: [u8; 4] = [0x11, 0x01, 0x00, 0x03];

/// Protocol Mode: Report Protocol (1) vs Boot Protocol (0)
pub const PROTOCOL_MODE_REPORT: u8 = 1;

/// HID Gamepad report (15 bytes of data, Xbox-compatible format).
///
/// NOTE: Report ID is NOT included in the data when using BLE HID with
/// Report Reference descriptor - the descriptor identifies the report.
///
/// Layout MUST match HID_REPORT_DESCRIPTOR exactly:
///   - 12 buttons + 4 padding (2 bytes)
///   - Hat switch (1 byte: 4 bits value + 4 bits padding)
///   - Left stick X/Y: 16-bit signed (4 bytes)
///   - Right stick X/Y: 16-bit signed (4 bytes)
///   - Triggers: 16-bit unsigned (4 bytes)
#[derive(Clone, Copy, Default)]
pub struct GamepadReport {
    /// Button states - 12 buttons in lower 12 bits (bits 0-11)
    /// Xbox layout: A,B,X,Y,LB,RB,Back,Start,L3,R3,Guide,unused
    pub buttons: u16,
    /// Hat switch / D-pad (0-7 for directions, 8 or 0x0F for neutral)
    /// 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW, 8/15=neutral
    pub hat: u8,
    /// Left stick X (-32768=left, 0=center, 32767=right)
    pub left_x: i16,
    /// Left stick Y (-32768=up, 0=center, 32767=down)
    pub left_y: i16,
    /// Right stick X (-32768=left, 0=center, 32767=right)
    pub right_x: i16,
    /// Right stick Y (-32768=up, 0=center, 32767=down)
    pub right_y: i16,
    /// Left trigger (0=released, 1023=fully pressed)
    pub left_trigger: u16,
    /// Right trigger (0=released, 1023=fully pressed)
    pub right_trigger: u16,
}

/// Hat switch values for D-pad directions.
pub mod hat {
    pub const NORTH: u8 = 0;
    pub const NORTH_EAST: u8 = 1;
    pub const EAST: u8 = 2;
    pub const SOUTH_EAST: u8 = 3;
    pub const SOUTH: u8 = 4;
    pub const SOUTH_WEST: u8 = 5;
    pub const WEST: u8 = 6;
    pub const NORTH_WEST: u8 = 7;
    pub const NEUTRAL: u8 = 8;  // Or 0x0F - no direction pressed
}

impl GamepadReport {
    /// Create a new report with neutral/centered values.
    pub fn new() -> Self {
        Self {
            buttons: 0,
            hat: hat::NEUTRAL,
            left_x: 0,           // Center
            left_y: 0,           // Center
            right_x: 0,          // Center
            right_y: 0,          // Center
            left_trigger: 0,     // Released
            right_trigger: 0,    // Released
        }
    }

    /// Convert to byte array for BLE transmission.
    /// Layout: buttons(2), hat(1), left_stick(4), right_stick(4), triggers(4) = 15 bytes
    /// All multi-byte values are little-endian.
    /// NOTE: Report ID is NOT included - Report Reference descriptor identifies the report.
    pub fn to_bytes(&self) -> [u8; 15] {
        [
            // Buttons (2 bytes: 12 buttons + 4 padding bits)
            (self.buttons & 0xFF) as u8,
            ((self.buttons >> 8) & 0x0F) as u8,  // Only lower 4 bits used, upper 4 are padding
            // Hat switch (1 byte: 4 bits value + 4 bits padding)
            self.hat & 0x0F,  // Lower 4 bits = hat value, upper 4 bits = padding (0)
            // Left stick X (2 bytes, little-endian)
            (self.left_x as u16 & 0xFF) as u8,
            ((self.left_x as u16 >> 8) & 0xFF) as u8,
            // Left stick Y (2 bytes, little-endian)
            (self.left_y as u16 & 0xFF) as u8,
            ((self.left_y as u16 >> 8) & 0xFF) as u8,
            // Right stick X (2 bytes, little-endian)
            (self.right_x as u16 & 0xFF) as u8,
            ((self.right_x as u16 >> 8) & 0xFF) as u8,
            // Right stick Y (2 bytes, little-endian)
            (self.right_y as u16 & 0xFF) as u8,
            ((self.right_y as u16 >> 8) & 0xFF) as u8,
            // Left trigger (2 bytes, little-endian)
            (self.left_trigger & 0xFF) as u8,
            ((self.left_trigger >> 8) & 0xFF) as u8,
            // Right trigger (2 bytes, little-endian)
            (self.right_trigger & 0xFF) as u8,
            ((self.right_trigger >> 8) & 0xFF) as u8,
        ]
    }
}

/// Button indices in the 16-bit button bitmask.
/// Xbox-compatible layout for BlueRetro/receiver compatibility:
/// - Bits 0-3: A, B, X, Y
/// - Bits 4-5: LB, RB (bumpers)
/// - Bit 6: Back/View
/// - Bit 7: Start/Menu
/// - Bits 8-9: L3, R3 (stick clicks)
/// - Bit 10: Guide
/// - Bits 11-14: D-pad (Up, Down, Left, Right)
pub mod buttons {
    pub const A: u16 = 1 << 0;
    pub const B: u16 = 1 << 1;
    pub const X: u16 = 1 << 2;
    pub const Y: u16 = 1 << 3;
    pub const LB: u16 = 1 << 4;      // Left bumper (unused on Dreamcast)
    pub const RB: u16 = 1 << 5;      // Right bumper (unused on Dreamcast)
    pub const BACK: u16 = 1 << 6;    // Back/View (unused on Dreamcast)
    pub const START: u16 = 1 << 7;   // Start/Menu
    pub const L3: u16 = 1 << 8;      // Left stick click (unused)
    pub const R3: u16 = 1 << 9;      // Right stick click (unused)
    pub const GUIDE: u16 = 1 << 10;  // Guide/Xbox button (unused)
    pub const DPAD_UP: u16 = 1 << 11;
    pub const DPAD_DOWN: u16 = 1 << 12;
    pub const DPAD_LEFT: u16 = 1 << 13;
    pub const DPAD_RIGHT: u16 = 1 << 14;
}

// GATT Service definitions using nrf-softdevice macros

/// HID Service (UUID 0x1812)
/// Security: JustWorks (encrypted, unauthenticated) - required by HOGP spec
#[nrf_softdevice::gatt_service(uuid = "1812")]
pub struct HidService {
    /// HID Information (UUID 0x2A4A) - Read only
    /// Value: [bcdHID_lo, bcdHID_hi, bCountryCode, flags]
    #[characteristic(uuid = "2A4A", read, security = "JustWorks")]
    pub hid_info: [u8; 4],

    /// Report Map (UUID 0x2A4B) - Read only, contains HID descriptor
    #[characteristic(uuid = "2A4B", read, security = "JustWorks")]
    pub report_map: Vec<u8, 128>,

    /// HID Report (UUID 0x2A4D) - Read, Notify (Input Report)
    /// Report Reference descriptor (0x2908): [Report ID=1, Report Type=Input(0x01)]
    #[characteristic(
        uuid = "2A4D",
        read,
        notify,
        security = "JustWorks",
        descriptor(uuid = "2908", security = "JustWorks", value = "[0x01, 0x01]")
    )]
    pub report: [u8; 15],

    /// HID Control Point (UUID 0x2A4C) - Write without response
    /// Used by host to signal suspend (0x00) or exit suspend (0x01)
    #[characteristic(uuid = "2A4C", write_without_response, security = "JustWorks")]
    pub control_point: u8,

    /// Protocol Mode (UUID 0x2A4E) - Read, Write Without Response
    /// 0 = Boot Protocol, 1 = Report Protocol (default)
    #[characteristic(uuid = "2A4E", read, write_without_response, security = "JustWorks")]
    pub protocol_mode: u8,
}

/// Device Information Service (UUID 0x180A)
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
#[nrf_softdevice::gatt_service(uuid = "180F")]
pub struct BatteryService {
    /// Battery Level (UUID 0x2A19) - 0-100%
    #[characteristic(uuid = "2A19", read, notify)]
    pub battery_level: u8,
}

/// Combined GATT server with all services.
#[nrf_softdevice::gatt_server]
pub struct GamepadServer {
    pub hid: HidService,
    pub device_info: DeviceInfoService,
    pub battery: BatteryService,
}

impl GamepadServer {
    /// Initialize the server with default values.
    pub fn init(&self) -> Result<(), SetValueError> {
        // Set HID Information
        self.hid.hid_info_set(&HID_INFO)?;

        // Set Report Map (HID descriptor)
        let mut report_map: Vec<u8, 128> = Vec::new();
        report_map.extend_from_slice(HID_REPORT_DESCRIPTOR).ok();
        self.hid.report_map_set(&report_map)?;

        // Set Protocol Mode to Report Protocol
        self.hid.protocol_mode_set(&PROTOCOL_MODE_REPORT)?;

        // Set initial report (neutral state)
        let initial_report = GamepadReport::new();
        self.hid.report_set(&initial_report.to_bytes())?;

        // Set Device Information
        let mut manufacturer: Vec<u8, 32> = Vec::new();
        manufacturer.extend_from_slice(b"Dreamcast").ok();
        self.device_info.manufacturer_set(&manufacturer)?;

        let mut model: Vec<u8, 32> = Vec::new();
        model.extend_from_slice(b"Controller").ok();
        self.device_info.model_number_set(&model)?;

        // PnP ID: Vendor ID Source, Vendor ID, Product ID, Version
        // Xbox One S Controller over BLE: VID=0x045E (Microsoft), PID=0x02E0
        let pnp_id: [u8; 7] = [
            0x02,       // Vendor ID Source (0x02 = USB-IF)
            0x5E, 0x04, // Vendor ID: 0x045E (Microsoft) - little endian
            0xE0, 0x02, // Product ID: 0x02E0 (Xbox One S BLE) - little endian
            0x00, 0x01, // Version 1.0
        ];
        self.device_info.pnp_id_set(&pnp_id)?;

        // Set battery level to 100% (we don't actually measure this)
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
