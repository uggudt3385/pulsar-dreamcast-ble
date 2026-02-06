# Maple Bus Implementation Learnings

Key lessons from implementing Maple Bus RX on nRF52840 at 2Mbps.

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

```rust
// Combined wait + sample - no function call gap
pub fn wait_and_sample(&mut self, timeout: u32) -> (...) {
    // Wait for start pattern...
    if a && b_transitions >= 3 {
        // IMMEDIATELY sample - don't return first!
        for i in 0..24576 {
            samples[i] = p0_in.read().bits();
        }
        return (true, ...);
    }
}

// Decode: sample AT edge, enforce phase
if last_a && !a {  // A fell
    let data_b = (samples[i] & PIN_B_MASK) != 0;  // Sample B at edge
    bits.push(data_b);
}
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
