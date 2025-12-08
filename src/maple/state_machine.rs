#[cfg(debug_assertions)]
use nrf52840_dk_bsp::embedded_hal::timer;

use crate::maple::{MaplePacket, traits::MapleBusTrait};
// use defmt_rtt as _;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapleState {
    Idle,
    WaitingForCommand,
    Receiving,
    Responding,
    Error,
}

pub struct MapleController<'bus_lifetime, B: MapleBusTrait> {
    state: MapleState,
    last_transaction_time: u64,
    bus: &'bus_lifetime mut B,
}

impl<'bus_lifetime, B: MapleBusTrait> MapleController<'bus_lifetime, B> {
    pub fn new(bus: &'bus_lifetime mut B) -> Self {
        Self {
            state: MapleState::Idle,
            last_transaction_time: 0,
            bus,
        }
    }

    pub fn step(&mut self, now_us: u64) {
        match self.state {
            MapleState::Idle => {
                if self.detect_start_signal() {
                    self.next_state(MapleState::WaitingForCommand);
                }
            }
            MapleState::WaitingForCommand => {
                if self.has_timed_out(now_us) {
                    self.next_state(MapleState::Error);
                } else if self.command_ready() {
                    self.next_state(MapleState::Receiving);
                }
            }
            MapleState::Receiving => {
                if self.receive_success() {
                    self.next_state(MapleState::Responding);
                } else {
                    self.next_state(MapleState::Error);
                }
            }
            MapleState::Responding => {
                let packet = MaplePacket::default();
                self.bus.write(&packet, false, 0);
                self.next_state(MapleState::Idle);
            }
            MapleState::Error => {
                self.reset_bus();
                self.next_state(MapleState::Idle);
            }
        }
    }

    fn detect_start_signal(&self) -> bool {
        /* GPIO check */
        false
    }
    fn command_ready(&self) -> bool {
        /* ready to read */
        false
    }
    fn receive_success(&self) -> bool {
        /* decode packet */
        false
    }
    fn send_response(&self) { /* GPIO write */
    }
    fn reset_bus(&self) { /* reset GPIO or timing */
    }

    fn has_timed_out(&self, now_us: u64) -> bool {
        now_us - self.last_transaction_time > 1000 // for example
    }

    fn next_state(&mut self, next: MapleState) {
        #[cfg(debug_assertions)]
        Self::log_transaction(&self, self.state, next, 0);
        self.state = next;
        self.last_transaction_time = 0;
    }

    fn log_transaction(&self, prev: MapleState, next: MapleState, now_us: u64) {
        // defmt::info!("[{}]State transition: {:?} -> {:?}", now_us, prev, next);
    }
}
