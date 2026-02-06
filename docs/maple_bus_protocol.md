# Maple Bus Protocol Reference

Consolidated from three sources:
- **[mc.pp.se]** http://mc.pp.se/dc/maplewire.html - Wire protocol details
- **[gmanmodz]** https://gmanmodz.com/2019/08/16/bit-banging-the-dreamcast-controller/ - Bit-banging implementation
- **[wiki]** https://dreamcast.wiki/Maple_bus - Complete protocol reference

---

## Physical Layer

### Pin Assignments

| Pin | Name | Color | Function |
|-----|------|-------|----------|
| 1 | SDCKA | Red | Clock/Data line A |
| 2 | +5V | Blue | Power |
| 3 | GND | Green | Ground (also sense) |
| 4 | GND | Black | Ground |
| 5 | SDCKB | White | Clock/Data line B |

*Pin/color mapping from physical inspection and community sources*

### Voltage Levels

| Source | Power | Signal Logic |
|--------|-------|--------------|
| [mc.pp.se] | +5V | +5V / 0V |
| [wiki] | 5V | 3.3V TTL |

**Note:** mc.pp.se describes 5V signaling, but wiki says 3.3V TTL. The Dreamcast itself uses 3.3V logic with 5V power. For nRF52840 (3.3V), this should be compatible.

### Idle State

**All sources agree:** Both SDCKA and SDCKB are pulled HIGH through weak pull-up resistors when idle.

- [mc.pp.se]: "When the bus is idle, both wires contain a high signal"
- [gmanmodz]: "Both data lines start at the high position"
- [wiki]: "Both lines on the Bus are pulled HIGH through weak pullup resistors"

---

## Timing

### Bit Timing

| Parameter | [mc.pp.se] | [wiki] | [gmanmodz] |
|-----------|------------|--------|------------|
| Phase duration | 0.5µs | ~160ns (host), ~250ns (peripheral) | - |
| Bit period | 1.0µs | - | - |
| Data rate | 2 Mbps | 2 Mbps (host), 0.5-1.3 Mbps (peripheral) | 1 MB/s |
| Edge transition (between lines) | - | ~125ns min | - |
| Edge transition (same line) | - | ~225ns min | - |

**Key insight from [wiki]:** Peripherals transmit slower than the host (~250ns/phase vs ~160ns/phase).

### Response Timing

| Parameter | Value | Source |
|-----------|-------|--------|
| Inter-chunk delay | 110-130µs between 4-word blocks | [wiki] |
| Device polling rate | 50Hz (PAL) / 60Hz (NTSC) | [wiki] |

---

## Start Pattern (Sync Sequence)

All three sources describe the same pattern:

### Sequence (Host sending to peripheral)

```
Initial state: SDCKA=HIGH, SDCKB=HIGH (idle)

1. SDCKA → LOW (immediately)
2. SDCKB toggled 4 times (HIGH→LOW cycles) while SDCKA stays LOW
3. SDCKB → HIGH
4. SDCKA → HIGH
5. SDCKB → LOW

Final state: SDCKA=HIGH, SDCKB=LOW (ready for Phase 1)
```

### Timing Diagram

```
         ___                           _____________
SDCKA:      \__________...____________/

         _______   ___   ___   ___   ___________
SDCKB:          \_/   \_/   \_/   \_/           \____

         idle  | A low | 4x B toggle | A,B rise | B low
```

### Detection (Receiver)

- [mc.pp.se]: "Four consecutive down flanks on pin 5 [SDCKB]" signals incoming frame
- [gmanmodz]: Uses interrupt on falling edge to detect start

### Bit Interpretation

[mc.pp.se]: "Interpreted as normal data transfer cycles, the sync appears as the bit sequence 100001, but with all bits but the first sent in phase 2."

---

## End Pattern (Terminate Sequence)

### Sequence

From [wiki]:
```
1. SDCKA → HIGH
2. SDCKB → HIGH then LOW
3. SDCKA toggled 2 times
4. SDCKB → HIGH

Final state: SDCKA=HIGH, SDCKB=HIGH (idle)
```

From [mc.pp.se]:
```
Bit sequence "100" sent as: Phase 2, Phase 1, Phase 1
Detection: Two consecutive Phase 1 cycles signal transmission end
```

---

## Data Transmission

### Phase Alternation

The protocol alternates which line is clock and which is data:

| Phase | Clock Line | Data Line | Clock Action |
|-------|------------|-----------|--------------|
| 1 | SDCKA | SDCKB | A falls → sample B |
| 2 | SDCKB | SDCKA | B falls → sample A |

**Critical insight from [mc.pp.se]:**
> "a receiving circuit need not concern itself with phases at all; a negative flank on any of the pins will always mean a valid bit on the other pin."

### Phase 1 Detailed (A=clock, B=data)

From [mc.pp.se]:
```
Start:  A=HIGH, B=LOW
1. B takes data bit value (B may go HIGH or stay LOW)
2. A driven LOW (falling edge = clock signal, data is now valid)
3. B stabilizes briefly
4. B driven HIGH (prepares B to be clock in next phase)
End:    A=LOW, B=HIGH (ready for Phase 2)
```

### Phase 2 Detailed (B=clock, A=data)

```
Start:  A=LOW, B=HIGH
1. A takes data bit value
2. B driven LOW (falling edge = clock signal)
3. A stabilizes briefly
4. A driven HIGH (prepares A to be clock in next phase)
End:    A=HIGH, B=LOW (ready for Phase 1)
```

### Data Sampling Rule

**When you see a falling edge on either pin, sample the OTHER pin immediately.**

---

## Bit/Byte/Word Ordering

### Bit Order

- [mc.pp.se]: MSB first (opposite of RS232)
- Bits transmitted: b7, b6, b5, b4, b3, b2, b1, b0

### Byte Order in Words

- [mc.pp.se]: "byte order reversal" - LSB transmitted first
- [wiki]: "little-endian bytes, MSB first"

For a 32-bit word `0xAABBCCDD`:
```
Transmission order: DD, CC, BB, AA (LSB first)
Within each byte: MSB first (bit 7 → bit 0)
```

### CRC Byte

- Single byte, MSB first
- **NOT** subject to byte reversal (it's just one byte)

---

## Frame Structure

### Frame Word (First Word of Packet)

| Byte Position | Field | Description |
|---------------|-------|-------------|
| Byte 0 (LSB, sent first) | Length | Payload word count (0-255) |
| Byte 1 | Sender | Source address |
| Byte 2 | Recipient | Destination address |
| Byte 3 (MSB, sent last) | Command | Command code |

As 32-bit value: `(command << 24) | (recipient << 16) | (sender << 8) | length`

### Complete Packet Structure

```
[Start Pattern] [Frame Word] [Payload Words 0..N-1] [CRC Byte] [End Pattern]
```

### CRC Calculation

Bytewise XOR of all bytes in the packet (frame word + all payload words):
```
crc = 0
for each word in packet:
    crc ^= (word >> 0) & 0xFF
    crc ^= (word >> 8) & 0xFF
    crc ^= (word >> 16) & 0xFF
    crc ^= (word >> 24) & 0xFF
```

---

## Addressing

### Port/Device Addresses

| Player | Host | Main Peripheral | Sub-1 | Sub-2 | Sub-3 | Sub-4 | Sub-5 |
|--------|------|-----------------|-------|-------|-------|-------|-------|
| 1 | 0x00 | 0x20 | 0x01 | 0x02 | 0x04 | 0x08 | 0x10 |
| 2 | 0x40 | 0x60 | 0x41 | 0x42 | 0x44 | 0x48 | 0x50 |
| 3 | 0x80 | 0xA0 | 0x81 | 0x82 | 0x84 | 0x88 | 0x90 |
| 4 | 0xC0 | 0xE0 | 0xC1 | 0xC2 | 0xC4 | 0xC8 | 0xD0 |

For our adapter (Player 1):
- Host (us): `0x00`
- Main controller: `0x20`
- VMU in slot 1: `0x01`
- VMU in slot 2: `0x02`

---

## Command Codes

### Common Commands

| Code | Name | Direction | Payload | Response |
|------|------|-----------|---------|----------|
| 0x01 | Device Info Request | Host→Device | 0 words | 0x05 |
| 0x05 | Device Info Response | Device→Host | 28 words | - |
| 0x09 | Get Condition | Host→Device | 1 word (function) | 0x08 |
| 0x08 | Data Transfer (Condition) | Device→Host | 3+ words | - |

### Device Info Request (0x01)

```
Frame: cmd=0x01, recipient=0x20, sender=0x00, length=0
Payload: none
```

### Device Info Response (0x05)

```
Frame: cmd=0x05, recipient=0x00, sender=0x20, length=28
Payload:
  Word 0: Function codes bitmask
  Word 1-3: Function definitions
  Word 4: Region code, direction, description start
  Words 5-27: Description, producer info, power consumption
```

### Get Condition (0x09)

```
Frame: cmd=0x09, recipient=0x20, sender=0x00, length=1
Payload:
  Word 0: Function code (0x00000001 for controller)
```

### Condition Response (0x08)

```
Frame: cmd=0x08, recipient=0x00, sender=0x20, length=3
Payload:
  Word 0: Function code
  Word 1: Buttons (active LOW - pressed = 0)
  Word 2: Triggers and analog stick
```

---

## Function Codes

| Code | Function |
|------|----------|
| 0x00000001 | Controller |
| 0x00000002 | Storage (VMU) |
| 0x00000004 | Screen (VMU LCD) |
| 0x00000008 | Timer (VMU clock) |
| 0x00000100 | Vibration (Rumble Pack) |

---

## Special Sequences

### Reset Sequence

```
SDCKA → LOW
SDCKB toggled 14 times
SDCKA → HIGH
```

### Light Gun Detection

```
SDCKA → LOW
SDCKB toggled 8 times
SDCKA → HIGH
```

---

## Implementation Notes

### For nRF52840 at 64MHz

| Parameter | Cycles |
|-----------|--------|
| 1 bit (1µs) | 64 cycles |
| Half bit (0.5µs) | 32 cycles |
| Start pattern (~4µs) | ~256 cycles |
| Typical response delay | 60-100µs = 4000-6400 cycles |

### Reception Strategy

From [gmanmodz]:
> "Since data is transmitted at 1MB/S, a fast microcontroller capable of accurately sampling bits is required—approximately 200 instructions available per bit at 200MHz."

At 64MHz, we have ~64 cycles per bit. Direct register access is essential.

### Key Implementation Points

1. **After start pattern ends (A=HIGH, B=LOW), immediately be ready to read**
   - Don't add delays - controller starts sending data right away

2. **Sample data on falling edge of clock line**
   - Phase 1: When A falls, sample B
   - Phase 2: When B falls, sample A

3. **Controller responds slower than host transmits**
   - Host: ~160ns/phase
   - Controller: ~250ns/phase
   - Give extra time when reading

---

## Real-World Implementation Insights

### From raphnet (AVR-based Dreamcast USB Adapter)
Source: https://www.raphnet.net/programmation/dreamcast_usb/index_en.php

**Critical Timing Discovery:**
- Documentation says 250ns/phase, but **data is only stable for ~240ns**
- This 10ns difference caused unreliable reads at slower clock speeds

**Clock Speed Requirements:**
- 12 MHz (83.3ns/cycle) had high error rate - not enough samples in 240ns window
- **16 MHz (62.5ns/cycle) worked reliably** - gets ~4 samples per stable window
- At 64 MHz (15.6ns/cycle), we should get ~15 samples per stable window

**Successful Implementation Strategy ("Memory Hungry But Fast"):**
```
Rather than detect edges in real-time, sample the port many times:
1. After detecting start pattern, immediately enter tight sampling loop
2. Sample GPIO register 640 times consecutively (assembly: in r16, PINC; st z+, r16)
3. Store all samples in RAM buffer
4. AFTER sampling complete, post-process to find edges and extract bits
```
This eliminates timing-critical decision-making during the receive window.

**Controller Response Time Variance:**
| Controller Type | Response Delay |
|-----------------|----------------|
| Official Sega | ~56 µs |
| Third-party | up to 159 µs |

**Voltage/Signal Issues:**
- 3.3V open-drain with pull-ups had problems due to cable capacitance
- Faster rise times needed - consider stronger pull-ups or active drive

### From ismell/maplebus (FPGA Implementation)
Source: https://github.com/ismell/maplebus

**Timing Constraints:**
- Controller must respond within **1 millisecond** of receiving command
- USB 2.0 latency (125µs minimum per transaction) made PC-based solutions difficult
- Hardware-level timing control (FPGA/dedicated MCU) is much more reliable

### From Gmanmodz (PIC32 Implementation)
Source: https://github.com/Gmanmodz/Dreamcast-Controller-Emulator

- PIC32 at 200 MHz gives ~200 instructions per bit
- Uses dedicated timing loops in C++
- Successfully emulates controller TO the Dreamcast (opposite direction from us)

### From dreamcast.wiki
Source: https://dreamcast.wiki/Maple_bus

**Additional Timing Details:**
- Host transmits at ~160ns/phase (faster)
- Peripherals transmit at ~250ns/phase (slower)
- Edge transition between lines: ~125ns minimum
- Edge transition on same line: ~225ns minimum
- Inter-chunk delay: 110-130µs between 4-word blocks

---

## Lessons for Our Implementation

### Why Real-Time Edge Detection is Hard
At 2 Mbps with ~240ns stable windows:
- Must detect falling edge AND sample other pin within ~240ns
- Any delay (function calls, branches, debug prints) can miss the window
- Polling loop overhead adds uncertainty

### Alternative: Bulk Sampling Approach
```rust
// Instead of:
loop {
    if pin_a_low() {
        bit = sample_pin_b();  // Timing critical!
        break;
    }
}

// Try:
let mut samples: [u32; 256] = [0; 256];
for i in 0..256 {
    samples[i] = gpio_register.read();  // Just sample, no decisions
}
// Now decode samples offline - find edges, extract bits
```

### Expected Waveform Characteristics
For Device Info Response from controller:
- Response delay: 50-160µs after our request ends
- Start pattern: A LOW, B toggles 4x, A HIGH, B LOW (~4µs)
- Data: 29 words × 32 bits × 500ns/bit = ~464µs
- Total response: ~500-600µs

---

## Our Current Issue (Updated)

**Status:** Can detect start pattern, reading data but misaligned by 1-2 bits

**Current Results:**
- Frame = 0x03000004
- cmd = 0x03 (expected 0x05, differ by 2 bits)
- Initial state after start pattern is now correct (A=1, B=0)

**Hypotheses:**
1. Edge detection timing is marginal - sometimes catching, sometimes missing
2. May need bulk sampling approach instead of real-time edge detection
3. Signal integrity issues - scope verification would help

**Next Steps:**
1. Use scope to verify actual waveform timing
2. Consider implementing bulk sampling approach
3. Try relaxed timing with small delays after edge detection
