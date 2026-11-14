<div align="center">
  <img src="docs/images/banner.png" alt="Aireluma — your iPhone on Windows" width="100%">
  <p><strong>Your iPhone. On your PC.</strong><br>Screen and sound over your local network.</p>
  <p>
    <a href="https://github.com/Anchoder/Aireluma/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/Anchoder/Aireluma?style=flat-square&color=7565e8"></a>
    <img alt="Windows 10 and 11, x64" src="https://img.shields.io/badge/Windows-10%20%2F%2011-4d78cc?style=flat-square">
    <a href="LICENSE"><img alt="GPL-3.0-or-later license" src="https://img.shields.io/badge/License-GPL--3.0--or--later-676777?style=flat-square"></a>
  </p>
  <p><a href="https://github.com/Anchoder/Aireluma/releases/latest/download/Aireluma-Setup.exe"><strong>Download for Windows</strong></a> · <a href="docs/README.ru.md">Русский</a> · <a href="https://github.com/Anchoder/Aireluma/issues">Report an issue</a></p>
</div>

## A small receiver for your desktop

Aireluma brings iPhone screen mirroring and audio to Windows through AirPlay. Open it, enable reception, and select Aireluma from Screen Mirroring on your iPhone.

| Feature | What you get |
| --- | --- |
| Screen and audio | Mirror your iPhone with sound on your PC |
| Up to 4K / 60 FPS | Request HEVC and a high-quality stream from your device |
| Light and dark | Switch appearance instantly; your choice is remembered |
| Local network | Reception works on your LAN; a wired PC is supported |
| System tray | Keep receiving after closing the control window |
| Native playback | A branded Direct3D12 viewer opens with the incoming stream |

Resolution, codec and frame rate are negotiated by the iPhone and depend on your device, network and computer. The requested maximum is not a guaranteed output mode.

## Two themes. One compact interface.

<img src="docs/images/themes.png" alt="Aireluma interface preview in light and dark themes" width="100%">

## Get connected

1. Download **Aireluma-Setup.exe** from the [latest release](https://github.com/Anchoder/Aireluma/releases/latest) and install it.
2. Open Aireluma and select **«Включить AirPlay»**.
3. Connect your iPhone and PC to the same local network. The PC can use Ethernet.
4. On your iPhone, open **Control Center → Screen Mirroring → Aireluma**.

The current interface is in Russian. Windows 10/11 x64 and Direct3D12 support are required. Close the window to keep Aireluma in the tray; use the tray menu to exit. Uninstall it through Windows Settings.

## Before opening an issue

- **Receiver missing:** check that both devices are on the same LAN and that the router is not isolating clients. Allow the receiver through the firewall only on a trusted network.
- **A particular video is blank:** some apps restrict screen capture or use a separate AirPlay media route. Test a local video in Photos to compare.
- **Frame rate varies:** source content, wireless conditions and device load affect the actual stream.

The installer is currently unsigned. A SHA-256 checksum is included with each release.

## Build and contribute

The interface is written in Rust using the Win32 API. The receiver is based on [UxPlay](https://github.com/FDH2/UxPlay), with GStreamer for playback. See [the build guide](docs/BUILDING.md) for dependencies, the pinned upstream source, and the published receiver patch.

Bug reports with Windows and iOS versions, a short reproduction, and whether Photos playback works are welcome. Avoid posting personal network information or private screen content.

## License and credits

[GPL-3.0-or-later](LICENSE). UxPlay and bundled libraries retain their respective copyright and license notices; see [NOTICE](NOTICE) and [the runtime package manifest](engine/runtime-packages.json). The release includes the installer, SHA-256 checksum and a source bundle.

AirPlay, iPhone and iOS are trademarks of Apple Inc. Aireluma is an independent project, not affiliated with or endorsed by Apple.
