# drempelbox

kids friendly audio system

## Building

### Using cross

This project uses [cross](https://github.com/cross-rs/cross/) for compiling to different target architectures.
Cross requires a regular rust install via [rustup](https://rustup.rs/), docker or podman, and a Linux kernel mit binmft_misc support, see also [cross dependencies](https://github.com/cross-rs/cross/#dependencies).

Then install cross using cargo:
```bash
cargo install cross --git https://github.com/cross-rs/cross
```

To build for 64bit RaspberryPi OS (aarch64-unknown-linux-gnu), use
```bash
make build-aarch64-unknown-linux-gnu
```

To build for build-x86_64-unknown-linux-gnu, use
```bash
make build-aarch64-unknown-linux-gnu
```

Cross uses a containerized build environment to make cross compilation easy, install any
external dependencies for the build step in (Dockerfile.cross)[Dockerfile.cross].

### Building directly on the host

You can also build this project directly on your host machine (for its architecture) using just cargo.
You will need to install the ALSA dev libs for your system, though (libasound2-dev on many platforms).

### Building on the RaspberryPi

***Don't do this unless you really need to! Compiling on the Zero 2W is very slow!***

To install the rust toolchain on the Zero 2W, you need to increase swap size.
From https://gist.github.com/tstellanova/0a6d8a70acc58a0d5be13ebaa7c935d4

```bash
sudo dphys-swapfile swapoff
sudo nano /etc/dphys-swapfile
```

Change `CONF_SWAPSIZE=100` to `CONF_SWAPSIZE=512` (or 1024)

```bash
sudo dphys-swapfile setup
sudo dphys-swapfile swapon
sudo reboot
```

Change to any swap size you feel appropriate for your needs.

## Running on RPi Zero 2W

Assuming a 64 bit RaspberryPI OS lite, install these packages to enable audio support:
- pipewire
- pipewire-alsa

Enable SPI for NFC ready support (using the raspi-config tool, for example).

## Hardware

Rough block diagram of system components:
```mermaid
classDiagram
    USB_PD_Decoy o-- Powerbank : USB A to USB C cable
    StepDownReg <|-- USB_PD_Decoy
    Amplifier  <|-- USB_PD_Decoy
    RaspberryPI_Zero_2W <|-- StepDownReg
    Speakers <|-- Amplifier
    USB_Soundcard <|-- RaspberryPI_Zero_2W
    Amplifier <|-- USB_Soundcard
     RaspberryPI_Zero_2W <|-- NFC_Module
    class NFC_Module {
    }
    class RaspberryPI_Zero_2W {
    }
    class USB_Soundcard {
    }
    class Powerbank{
        USB PD
        SoC Display
        USB C in/out
        USB Micro in
        USB A out
        charge()
        discharge()
    }
    class USB_PD_Decoy{
        12V
    }
    class StepDownReg{
        5V
    }
    class Amplifier{
    }
    class Speakers{
    }
```
