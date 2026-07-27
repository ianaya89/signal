## Install

**One-liner (macOS · Linux)**

```sh
curl -fsSL https://ianaya89.github.io/signal/install.sh | sh
```

**Or grab an asset below**

| File | Notes |
|---|---|
| `Signal_*_arm64.dmg` / `_x86_64.dmg` | macOS 11+. libmpv is bundled — nothing else to install. Unsigned: first launch needs right-click → Open, or `xattr -dr com.apple.quarantine /Applications/Signal.app`. |
| `Signal_*_amd64.deb` | Ubuntu 24.04+ / Debian 13+ (needs `libmpv2`). `apt install ./Signal_*.deb` pulls it in. |
| `Signal_*_amd64.AppImage` | Any glibc 2.35+ distro, including older Ubuntu. `chmod +x` and run — libmpv travels inside the image. |

The `signal` CLI is not shipped as a binary yet — build it with
`cargo build -p signal-cli`.

## Changes

<!-- filled in before publishing -->
