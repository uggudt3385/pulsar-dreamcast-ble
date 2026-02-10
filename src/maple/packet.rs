use heapless::Vec;

/// Represents a Maple Bus packet ready to send or parse.
pub struct MaplePacket {
    pub sender: u8,
    pub recipient: u8,
    pub command: u8,
    pub payload: Vec<u32, 255>, // up to 255 u32 words
}

impl MaplePacket {
    /// Builds the 32-bit frame word used at the start of the packet.
    /// Format: `[length:8][sender:8][recipient:8][command:8]`
    /// Byte 0 = length, Byte 1 = sender, Byte 2 = recipient, Byte 3 = command
    #[must_use]
    pub fn frame_word(&self) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // Payload max 255 words
        let num_words = self.payload.len() as u32;
        (u32::from(self.command) << 24)
            | (u32::from(self.recipient) << 16)
            | (u32::from(self.sender) << 8)
            | (num_words & 0xFF)
    }
}
