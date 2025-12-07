use crate::maple::{MaplePacket, bus::BusStatus};

pub trait MapleBusTrait {
    fn write(&mut self, packet: &MaplePacket, autostart_read: bool, timeout_us: u64) -> bool;
    fn process_events(&mut self, now_us: u64) -> BusStatus;
}
