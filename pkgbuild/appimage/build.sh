#!/usr/bin/env bash
set -euo pipefail

if [[ $# != 2 ]]; then
    printf 'Usage: bash %s /path/to/appimagetool-x86_64.AppImage /path/to/release/keyboard-tester\n' "$0" >&2
    exit 2
fi

root=$(cd -- "$(dirname -- "$0")/../.." && pwd)
tool=$(realpath -- "$1")
binary=$(realpath -- "$2")
output="$root/target/Keyboard_Tester-x86_64.AppImage"
mkdir -p "$(dirname -- "$output")"
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
appdir="$scratch/Keyboard_Tester.AppDir"

install -Dm755 "$binary" "$appdir/usr/bin/keyboard-tester"
install -Dm755 "$root/pkgbuild/appimage/AppRun" "$appdir/AppRun"
install -Dm644 "$root/pkgbuild/keyboard-tester.desktop" "$appdir/keyboard-tester.desktop"
install -Dm644 "$root/pkgbuild/keyboard-tester.svg" "$appdir/keyboard-tester.svg"
install -Dm644 "$root/LICENSE" "$appdir/usr/share/licenses/keyboard-tester/LICENSE"
ln -s keyboard-tester.svg "$appdir/.DirIcon"

ARCH=x86_64 "$tool" --appimage-extract-and-run --no-appstream "$appdir" "$output"
