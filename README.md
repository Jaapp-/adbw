`adbw` wraps `adb` for connecting to a wireless device quickly.

Requires `adb` on PATH from Platform Tools 36.0.2+ with mDNS support.
Phone and computer must be on the same network.

```sh
cargo install adbw
```


```console
$ adbw  # Turn on wireless debugging on Android device
Scan in Wireless debugging > Pair device with QR code:
█████████████████████████████████████████████
█████████████████████████████████████████████
████ ▄▄▄▄▄ ███ ▀▄█▄▄ ▀ ▀▄▀█▄▄▀▀ ▄█ ▄▄▄▄▄ ████
████ █   █ █ ▀▀▀▀▀▄██▄▄  ▄█▄ ▄█▄██ █   █ ████
████ █▄▄▄█ █ ▄███▀▄▀██ ▄▄▄  ▀▄▄ ▄█ █▄▄▄█ ████
████▄▄▄▄▄▄▄█ █▄▀▄█ ▀ █▄▀▄▀▄█ █ █ █▄▄▄▄▄▄▄████
████ ▀▄▄▄▄▄▀▀▄▄▄█▄█ ▀▀▄█▄▄  ██▀ ▄█▄ ▄▄▄██████
████▄█▀▄▄ ▄ █▄▄▄▀█▄█▀▄ █▄██▀▀ ▄▀ ▀▄█▀▄▄  ████
████▀ ▄▀▄ ▄  ▄ ▀▀███▀ █▀  ▄█▄    ▀█▀▀  ▄▄████
█████▄   █▄▀  █  ▄  █ ▀███▀  ██▄ █ ███ ▄▄████
███████ ▀▄▄▄▄▄▀ █▀▄▀▄█▀▄▀█▀ ██▄ ▀▄▀▀▄█▄█▄████
█████▀ █▀ ▄▀█▄▄██▄█ ▄ ▀ ▀▄▄▀▀ █ █  █▄▄▀▀▄████
████▀█▄▀█▀▄ ▄▀▀▀  ▀ ▀▄ ▄▀ ▄▀▄█  ▀▀▄▀█▄█▀█████
████▄█▀█▀█▄ ▀ ███▄▀▀██▄▄▀█▀█▀▀█▄ ██▀█▄▄▄ ████
████▀█▀█▀ ▄ ██▀ ▄   ▀█▄▄▄█▀███▀▄█▄▄▀▀▀  ▀████
████ █▀▀▀▄▄▀ ▀▀█▄ ▄█▀█ █▄ █▀▄ ▀ ▀▀▄█▄▄▄  ████
████▄█████▄█▀▄▄▀▄▄ █  ██▀▀▄█ ██▄ ▄▄▄ █ ▄ ████
████ ▄▄▄▄▄ █▀▄█▀██▀ █ ▀ █ ██▀▄▄▀ █▄█  ▄▄▀████
████ █   █ █  ▀▀▄█ ▀▄█▀█▀ ██▄▄▀▄ ▄▄ ▄▀▄▄▀████
████ █▄▄▄█ █▄▄▀▄ ▀▄ ▄█▀ ▀▀▄▀▄▄▄▀█▀█▄▀▄▄█▄████
████▄▄▄▄▄▄▄█▄██▄▄▄▄▄█▄█▄▄█▄█████▄█▄███▄█▄████
█████████████████████████████████████████████
▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
Connected to adb-XXXXXXXXXXXXXXX-XXXXXX._adb-tls-connect._tcp
```

When already paired:

```console
$ adbw
Connected to adb-XXXXXXXXXXXXXXX-XXXXXX._adb-tls-connect._tcp
```

Connections use the mDNS service name so ADB reuses the same device entry as its
automatic mDNS connection.

Troubleshooting: `adbw --debug`.

