# drempelbox

kids friendly audio system

## Rust on RPi Zero 2W

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

## Required packages for developing (debian):

- libasound2-dev
- libdbus-1-dev
- libssl-dev
- pkg-config

## Required packages for running (debian):

- pipewire
- pipewire-alsa
