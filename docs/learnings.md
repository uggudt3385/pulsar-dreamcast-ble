# Maple Bus Implementation Learnings

Key lessons from implementing Maple Bus RX on nRF52840 at 2Mbps.

---

## Hardware Setup (Working Configuration)

### Pin Assignments
- **P0.05** = SDCKA (Red wire)
- **P0.06** = SDCKB (White wire)
- **5V** = Controller power (from external supply or DK VDD)
- **GND** = Controller ground + Sense pin (Green wire)

### Pull-ups
- 4.7kΩ pull-ups from both data lines to 3.3V
- Controller signals are 3.3V TTL compatible

### Idle State
- SDCKA = HIGH
- SDCKB = LOW
- If you see the opposite at startup (A=0, B=1), **check wiring** - cables may be swapped

---

## Critical Timing Lessons

### 1. Every Microsecond Counts
At 2Mbps, one bit = 500ns. A single `rprintln!()` costs 1000+ cycles (~15µs) = 30 bits missed. Debug prints in the critical path will corrupt your data.

### 2. Function Call Overhead Matters
Returning from `wait_for_start()` then calling `bulk_sample()` added enough delay to miss the first clock edge. **Solution:** Combine detection and sampling in one function - start sampling inline the moment you detect the trigger condition.

### 3. Stack Allocation is Not Free
Allocating a 96KB buffer on the stack (`[u32; 24576]`) caused measurable delay. By the time allocation completed, data transmission was already in progress. **Solution:** Use `static mut` buffers - pre-allocated at startup, zero runtime cost.

---

## Protocol Implementation Lessons

### 4. Sample AT the Edge, Not After
Original code: `samples[i+1]` (one sample after detecting edge)
Fixed code: `samples[i]` (at the edge)

At ~15 samples per bit, even one sample delay can catch the data line mid-transition.

### 5. Phase Alignment is Critical
If your first detected edge is B falling (Phase 2) instead of A falling (Phase 1), all subsequent bits are shifted by one position. **Solution:** Skip edges until you see the first A fall, ensuring correct phase.

### 6. Bulk Sampling > Real-Time Edge Detection
Real-time edge detection requires making decisions in <500ns windows. Bulk sampling (capture everything, decode later) eliminates timing-critical logic during receive.

---

## Hardware/Tooling Lessons

### 7. Check for Blocking Processes Before Flashing
VS Code extensions (nRF Connect), stale probe-rs processes, and JLink GUI can hold the debugger connection. Always run:
```bash
ps aux | grep -iE 'jlink|probe-rs|nrf' | grep -v grep
```

### 8. Pull-ups Are Essential
Both SDCKA and SDCKB need pull-ups to 3.3V. Without them, floating lines cause false edge detection.

### 9. Ground the Sense Pin
Dreamcast controllers won't respond unless the GND/Sense pin (Green wire) is connected to ground.

---

## Debugging Strategies That Worked

### 10. Capture Raw Data First, Analyze Later
Bulk sampling let us see exactly what the signals looked like, independent of our decode logic. This separated "is the signal there?" from "are we decoding it right?"

### 11. Add Diagnostic Counters
Counting B transitions during start pattern (expected: 8) immediately revealed when we were detecting false starts. Simple counters catch problems faster than trying to interpret corrupted data.

### 12. Compare Expected vs Actual Bit-by-Bit
When frame bytes were wrong, comparing individual bits showed patterns (e.g., "5 of 8 bits match, shifted by 1") that pointed to phase alignment issues.

---

## The Winning Configuration

### Key Parameters (DO NOT CHANGE)
```rust
// In wait_and_sample():
b_transitions >= 3    // Minimum B transitions to accept start pattern

// In read_packet_bulk():
find_first_edges(samples, count, 40)  // Analyze 40 edges for late-start detection
                                       // DO NOT reduce to 10 - causes decode failures!

// Buffer size:
static mut SAMPLE_BUFFER: [u32; 24576]  // 96KB static buffer
```

### Combined Wait + Sample Pattern
```rust
pub fn wait_and_sample(&mut self, timeout: u32) -> (...) {
    // Wait for idle (both HIGH), then A LOW, count B transitions
    if a && b_transitions >= 3 {
        // IMMEDIATELY sample - no function return delay!
        compiler_fence(Ordering::SeqCst);
        for i in 0..24576 {
            samples[i] = p0_in.read().bits();
        }
        compiler_fence(Ordering::SeqCst);
        return (true, ...);
    }
}
```

### Phase-Aligned Decoding
```rust
// In decode_bulk_samples():
if last_a && !a {  // A fell (Phase 1)
    seen_first_a_fall = true;
    bits.push((samples[i] & PIN_B_MASK != 0) as u8);  // Sample B AT edge
}
else if last_b && !b {  // B fell (Phase 2)
    if seen_first_a_fall {  // Only after first A fall!
        bits.push((samples[i] & PIN_A_MASK != 0) as u8);  // Sample A AT edge
    }
}
```

---

## Common Regressions to Avoid

### 13. Edge Count for Late-Start Detection
The `find_first_edges()` call must analyze **40 edges**, not 10. Reducing this causes the late-start detection to fail, resulting in garbage frame data even though sampling works.

```rust
// CORRECT:
let edges = self.find_first_edges(samples, count, 40);

// WRONG - causes decode failures:
let edges = self.find_first_edges(samples, count, 10);
```

### 14. Verify Wiring with Initial State
At startup, before configuring outputs, read pins as pull-up inputs:
```
Expected idle: A=1 B=0
If you see:    A=0 B=1  → Wires are swapped!
If you see:    A=0 B=0  → Controller not powered or not connected
```

---

## Quick Reference

| Problem | Symptom | Solution |
|---------|---------|----------|
| Debug prints in critical path | Garbage data, missed bits | Remove all prints before sampling |
| Stack allocation delay | Initial state A=0 B=0 | Use static buffer |
| Function return delay | First edge is B not A | Combine wait+sample |
| Sampling after edge | Bit values off by ~1 | Sample at `samples[i]` |
| Wrong phase start | Bytes shifted by 1 bit | Skip B edges until first A fall |
| Probe "not found" | Can't flash | Kill nrfutil-device/probe-rs processes |
| find_first_edges too small | Frame=garbage, b_trans OK | Use 40 edges, not 10 |
| Initial state A=0 B=1 | No response (b_trans=0) | Wires swapped - check Red→P0.05, White→P0.06 |
| Core locked | Flash fails with lock error | `probe-rs erase --chip nRF52840_xxAA --allow-erase-all` |

---

## Expected Working Output

When everything is correct, you should see:
```
Initial bus state (as inputs): A=1 B=0
...
TX: DeviceInfoRequest
RX: Frame=0x0500201C cmd=0x05 len=28
RX: OK!
Controller detected!
  Functions: 0x00000001
```

Key indicators:
- **A=1 B=0** at startup (correct idle state)
- **Frame=0x0500201C** = Device Info Response (cmd=0x05, len=28, sender=0x20, recipient=0x00)
- **Functions: 0x00000001** = Standard controller
