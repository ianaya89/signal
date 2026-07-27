#!/usr/bin/env bash
# Folds libmpv and its non-system dependencies into the AppImage that
# `tauri build` just produced, then repacks it.
#
# The .deb can simply declare `libmpv2 | libmpv1`, but an AppImage is supposed
# to run on a distro that never heard of mpv — and Tauri's bundler does not know
# libmpv is needed, because the binary picks it up through pkg-config at link
# time. Without this step the AppImage aborts on launch wherever libmpv.so.2 is
# absent.
#
#   ./scripts/appimage-bundle-mpv.sh
set -euo pipefail
cd "$(dirname "$0")/.."

BUNDLE=target/release/bundle/appimage
APPDIR="$(find "$BUNDLE" -maxdepth 1 -name '*.AppDir' -type d | head -1)"
ORIGINAL="$(find "$BUNDLE" -maxdepth 1 -name '*.AppImage' -type f | head -1)"

[ -n "$APPDIR" ] || { echo "warning: no AppDir under ${BUNDLE} — skipping mpv bundling" >&2; exit 0; }
[ -n "$ORIGINAL" ] || { echo "warning: no AppImage under ${BUNDLE} — skipping mpv bundling" >&2; exit 0; }

LIBDIR="${APPDIR}/usr/lib"
mkdir -p "$LIBDIR"

MPV="$(pkg-config --variable=libdir mpv)/libmpv.so.2"
[ -f "$MPV" ] || MPV="$(ldconfig -p | awk '/libmpv\.so\.2/ {print $NF; exit}')"
[ -f "$MPV" ] || { echo "error: libmpv.so.2 not found" >&2; exit 1; }

# Anything provided by every glibc system stays out: bundling these is the
# classic way to make an AppImage crash on a newer host.
SKIP='^(linux-vdso|ld-linux|libc|libm|libdl|libpthread|librt|libresolv|libgcc_s|libstdc\+\+)\.'

copy_closure() {
  local lib="$1"
  # `&& continue` would trip set -e inside the loop body, so branch explicitly.
  while read -r soname path; do
    if [ ! -f "${path:-}" ]; then continue; fi
    if [[ "$soname" =~ $SKIP ]]; then continue; fi
    if [ -f "${LIBDIR}/${soname}" ]; then continue; fi
    cp -L "$path" "${LIBDIR}/${soname}"
    copy_closure "${LIBDIR}/${soname}"
  done < <(ldd "$lib" | awk '{print $1, $3}')
}

cp -L "$MPV" "${LIBDIR}/libmpv.so.2"
copy_closure "${LIBDIR}/libmpv.so.2"
echo "AppDir now carries $(find "$LIBDIR" -name '*.so*' | wc -l | tr -d ' ') shared libraries"

# appimagetool repacks the AppDir as it stands; linuxdeploy would re-run its own
# dependency logic and drop what we just added.
TOOL="${BUNDLE}/appimagetool"
if [ ! -x "$TOOL" ]; then
  echo "fetching appimagetool"
  curl -fsSL -o "$TOOL" \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$(uname -m).AppImage"
  chmod +x "$TOOL"
fi

mv "$ORIGINAL" "${ORIGINAL}.orig"
if APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$(uname -m)" "$TOOL" "$APPDIR" "$ORIGINAL"; then
  rm -f "${ORIGINAL}.orig"
  echo "repacked: ${ORIGINAL}"
else
  # Better to ship the original than nothing; the release notes tell users to
  # install libmpv in that case.
  mv "${ORIGINAL}.orig" "$ORIGINAL"
  echo "warning: repack failed — keeping the AppImage without libmpv bundled" >&2
  exit 0
fi
