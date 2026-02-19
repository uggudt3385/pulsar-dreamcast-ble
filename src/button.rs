//! Sync button monitoring task.

use embassy_nrf::gpio::{Input, Output};
use embassy_time::{Duration, Instant, Timer};
use rtt_target::rprintln;

use crate::ble::{get_connection_state, ConnectionState};
use crate::{NAME_TOGGLE, SYNC_MODE};

/// Sync button monitoring task.
///
/// - Hold 3 seconds: enter pairing/sync mode
/// - Triple-press within 2 seconds: toggle device name (Xbox <-> Dreamcast) and reset
///
/// LED1 behavior based on `ConnectionState`:
/// - `Idle`/`Reconnecting`: OFF
/// - `SyncMode`: Fast blink (200ms on/off)
/// - `Connected`: Solid ON
#[allow(clippy::items_after_statements)]
#[embassy_executor::task]
pub async fn sync_button_task(button: Input<'static>, mut led: Output<'static>) {
    const HOLD_DURATION_MS: u64 = 3000;
    const BLINK_INTERVAL_MS: u64 = 100;
    const TRIPLE_PRESS_WINDOW_MS: u64 = 2000;

    // Let pull-up settle before reading button state
    Timer::after(Duration::from_millis(100)).await;
    rprintln!("BTN: pin={}", if button.is_high() { "HIGH" } else { "LOW" });

    let mut press_count: u8 = 0;
    let mut first_press_time = Instant::now();

    let mut last_logged_state = ConnectionState::Idle;
    loop {
        let state = get_connection_state();

        // Log state transitions
        if state != last_logged_state {
            rprintln!("BTN: state={:?}", state);
            last_logged_state = state;
        }

        // Update LED based on state
        match state {
            ConnectionState::Connected => {
                led.set_low(); // LED on (active low)
            }
            ConnectionState::SyncMode => {
                // Fast blink handled below when not checking button
                led.set_low();
                Timer::after(Duration::from_millis(200)).await;
                led.set_high();
                Timer::after(Duration::from_millis(200)).await;

                // Check for button press to cancel sync mode early
                if button.is_low() {
                    Timer::after(Duration::from_millis(100)).await;
                    if button.is_low() {
                        rprintln!("SYNC: Cancelled by button press");
                        while button.is_low() {
                            Timer::after(Duration::from_millis(50)).await;
                        }
                    }
                }
                continue; // Skip the button hold detection below
            }
            ConnectionState::Idle | ConnectionState::Reconnecting => {
                led.set_high(); // LED off
            }
        }

        // Check for button press (active low)
        if button.is_high() {
            // Reset triple-press counter if window expired
            if press_count > 0 && first_press_time.elapsed().as_millis() >= TRIPLE_PRESS_WINDOW_MS {
                press_count = 0;
            }
            Timer::after(Duration::from_millis(50)).await;
            continue;
        }

        rprintln!("BTN: press detected");

        // Button pressed - start timing with LED feedback
        let press_start = Instant::now();
        let mut led_state = false;
        let mut last_blink = Instant::now();
        let mut held_long = false;

        // Wait for either release or hold duration
        while button.is_low() {
            // Blink LED while holding to indicate pending action
            if last_blink.elapsed().as_millis() >= BLINK_INTERVAL_MS {
                led_state = !led_state;
                if led_state {
                    led.set_low(); // LED on
                } else {
                    led.set_high(); // LED off
                }
                last_blink = Instant::now();
            }

            if press_start.elapsed().as_millis() >= HOLD_DURATION_MS {
                // Held long enough - trigger sync mode
                held_long = true;
                rprintln!("SYNC: Entering pairing mode (60s)");
                SYNC_MODE.signal(());
                press_count = 0; // Reset triple-press counter

                // Wait for button release
                while button.is_low() {
                    led.set_low();
                    Timer::after(Duration::from_millis(100)).await;
                    led.set_high();
                    Timer::after(Duration::from_millis(100)).await;
                }
                break;
            }
            Timer::after(Duration::from_millis(20)).await;
        }

        if !held_long {
            // Short press — count for triple-press detection
            if press_count == 0 {
                first_press_time = Instant::now();
            }
            press_count += 1;

            if press_count >= 3 && first_press_time.elapsed().as_millis() < TRIPLE_PRESS_WINDOW_MS {
                // Triple press detected! Toggle name preference.
                let current = crate::ble::flash_bond::load_name_preference();
                let new_pref = !current;
                rprintln!(
                    "NAME: Triple-press! Switching to {}",
                    if new_pref { "Dreamcast" } else { "Xbox" }
                );

                // LED confirmation: 5 rapid blinks
                for _ in 0..5 {
                    led.set_low();
                    Timer::after(Duration::from_millis(50)).await;
                    led.set_high();
                    Timer::after(Duration::from_millis(50)).await;
                }

                // Signal ble_task to save and reset
                NAME_TOGGLE.signal(new_pref);
                press_count = 0;
            }
        }

        // Debounce
        Timer::after(Duration::from_millis(100)).await;
    }
}
