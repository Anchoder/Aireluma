#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)
revision=$(sed -n 's/.*"revision": "\([^"]*\)".*/\1/p' engine/upstream.json)
if [[ ! -d engine/uxplay/.git && ! -f engine/uxplay/.source-revision ]]; then
  git clone https://github.com/FDH2/UxPlay.git engine/uxplay
  git -C engine/uxplay checkout --detach "$revision"
  git -C engine/uxplay apply ../aireluma.patch
fi
cargo build --release --locked
cmake -S engine/uxplay -B engine/uxplay/build -G Ninja -DCMAKE_BUILD_TYPE=Release -DNO_MARCH_NATIVE=ON -DNO_X11_DEPS=ON
cmake --build engine/uxplay/build --parallel 2
stage="artifacts/deb-$version"
mkdir -p "$stage/usr/bin" "$stage/usr/lib/aireluma" "$stage/usr/share/applications" "$stage/usr/share/icons/hicolor/256x256/apps" "$stage/usr/share/doc/aireluma" "$stage/DEBIAN"
install -m755 target/release/airplay-windows "$stage/usr/bin/aireluma"
install -m755 engine/uxplay/build/uxplay "$stage/usr/lib/aireluma/uxplay"
install -m644 assets/aireluma.png "$stage/usr/share/icons/hicolor/256x256/apps/io.github.Anchoder.Aireluma.png"
install -m644 packaging/linux/io.github.Anchoder.Aireluma.desktop "$stage/usr/share/applications/"
install -m644 LICENSE "$stage/usr/share/doc/aireluma/copyright"
install -m644 NOTICE "$stage/usr/share/doc/aireluma/NOTICE"
install -m644 engine/uxplay/LICENSE "$stage/usr/share/doc/aireluma/UxPlay-LICENSE"
mkdir -p artifacts/shlibs/debian
printf 'Source: aireluma\nSection: video\nPriority: optional\nMaintainer: Aireluma contributors\nStandards-Version: 4.6.2\n\nPackage: aireluma\nArchitecture: any\nDescription: AirPlay screen and audio receiver\n' > artifacts/shlibs/debian/control
project_root="$PWD"
dependencies=$(cd artifacts/shlibs && dpkg-shlibdeps -O -e"$project_root/$stage/usr/bin/aireluma" -e"$project_root/$stage/usr/lib/aireluma/uxplay" | sed 's/^shlibs:Depends=//')
cat > "$stage/DEBIAN/control" <<EOF
Package: aireluma
Version: $version
Section: video
Priority: optional
Architecture: $(dpkg --print-architecture)
Maintainer: Aireluma contributors <352085178+Anchoder@users.noreply.github.com>
Depends: $dependencies, gstreamer1.0-plugins-base, gstreamer1.0-plugins-good, gstreamer1.0-plugins-bad, gstreamer1.0-libav
Homepage: https://github.com/Anchoder/Aireluma
Description: AirPlay screen and audio receiver for iPhone
 A compact native Rust interface with light and dark themes.
EOF
dpkg-deb --root-owner-group --build "$stage" "artifacts/Aireluma_${version}_$(dpkg --print-architecture).deb"
