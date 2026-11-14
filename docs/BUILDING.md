# Building Aireluma

The interface uses Rust and the Win32 API. Reception runs in a separate UxPlay process, with GStreamer handling Direct3D12 video and audio playback.

## Tools

- Windows 10 or 11, x64.
- Git and Rust installed with rustup.
- [MSYS2](https://www.msys2.org/) with the UCRT64 environment.
- [Inno Setup 6](https://jrsoftware.org/isinfo.php) to build the installer.

In the MSYS2 UCRT64 terminal, install the build dependencies:

```sh
pacman -S --needed mingw-w64-ucrt-x86_64-toolchain mingw-w64-ucrt-x86_64-cmake mingw-w64-ucrt-x86_64-ninja mingw-w64-ucrt-x86_64-pkgconf mingw-w64-ucrt-x86_64-openssl mingw-w64-ucrt-x86_64-libplist mingw-w64-ucrt-x86_64-gstreamer mingw-w64-ucrt-x86_64-gst-plugins-base mingw-w64-ucrt-x86_64-gst-plugins-good mingw-w64-ucrt-x86_64-gst-plugins-bad mingw-w64-ucrt-x86_64-gst-plugins-ugly mingw-w64-ucrt-x86_64-gst-libav
```

From PowerShell in the repository root:

```powershell
./scripts/build.ps1 -Installer
```

For a different MSYS2 location:

```powershell
./scripts/build.ps1 -Msys2Root D:/msys64 -Installer
```

The script fetches the pinned UxPlay revision, applies the published patch, builds both executables without host-specific CPU tuning, collects runtime files and licenses, and creates `installer/Aireluma-Setup.exe`. The runnable application is `package/airplay-windows.exe`.

## Runtime versions

`engine/runtime-files.txt` records the runtime filenames used by the published installer. `engine/runtime-packages.json` records package versions and source references. MSYS2 is a rolling distribution: when an ABI changes, update the runtime manifest together with the corresponding packages. The build stops if an expected file is missing.

## Receiver changes

The upstream revision and source URL are recorded in `engine/upstream.json`. `engine/aireluma.patch` contains the complete Aireluma receiver changes: network adapter selection for discovery, stream-state events, and Windows application identity. Original third-party copyright and license notices remain intact.

## Themes

The theme switch applies immediately and saves the preference in `%APPDATA%/Aireluma/appearance`. A fresh installation uses the light theme. Theme changes do not restart the receiver.
