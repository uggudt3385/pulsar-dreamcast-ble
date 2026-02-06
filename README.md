# nrf52840 setup

This project is currently using s140, it needs to be reflashed onto the chip when the chip is completely erased. 

This is a basic setup of the nrf52840 DK following the example of this video

https://youtu.be/TOAynddiu5M?si=5fiKa9r9CBKWlAwq

## Features

- Set up tomls
- gdb and rtt options
- increments number

## Getting Started

### Basic Use

```
cargo embed
```

### GDB 
After starting cargo embed, in another terminal run:
```
arm-none-eabi-gdb target/thumbv7em-none-eabihf/debug/embedded_rust_setup
```
Once connected to the target,
```
target remote :1337
```

Useful GDB commands:
```
info registers
disassemble
stepi
break [file]:[line]
continue
info locals
print [var]
set var [var]=[value]
info break
delete [break point number]
monitor reset
```


### RTT (Real-Time Transfer)

RTT lets you print debug messages from embedded code via `rprintln!()`.

**Quick start:**
```bash
cargo embed --bin embedded_rust_setup
```
This flashes and opens RTT automatically. Output appears in terminal.

**In code:**
```rust
use rtt_target::{rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    rtt_init_print!();  // Initialize once at startup
    rprintln!("Hello from embedded!");
    loop { /* ... */ }
}
```

**View RTT only (already flashed):**
```bash
probe-rs attach --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/embedded_rust_setup
```

**Timing warning:** `rprintln!()` takes ~15µs. Don't use in timing-critical paths!

**Troubleshooting - probe not found:**
```bash
ps aux | grep -iE 'jlink|probe-rs|nrf' | grep -v grep
kill <pid>  # Kill any blocking processes
```

**Chip locked:**
```bash
probe-rs erase --chip nRF52840_xxAA --allow-erase-all
```

### Installation

Identify architecture via data sheet - Microcontroller
Identify Processor (Arm Cortex M4 w/ FPU), then architecture (Armv7E-M), followed by instruction set (Thumb/Thumb2)

Search rust platform-support page for architecture, we will use the one with the hard float due to it having an FPUq
thumbv7em-none-eabihf

Manually add using:
```
rustup target add thumbv7em-none-eabihf
rustup show
```
### Useful links
Website: https://www.nordicsemi.com/Products/Development-hardware/nRF52840-DK/Download
DataSheet: https://www.mouser.com/datasheet/2/297/nRF52840_PS_v1.1-1623672.pdf
Schematic: https://os.mbed.com/platforms/Nordic-nRF52840-DK/