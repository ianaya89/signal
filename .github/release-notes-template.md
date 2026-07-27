## Install

**One-liner (macOS · Linux)**

```sh
curl -fsSL https://ianaya89.github.io/signal/install.sh | sh
```

**Or grab an asset below**

| File | Notes |
|---|---|
| `Signal_*_arm64.dmg` / `_x86_64.dmg` | macOS 11+. libmpv is bundled — nothing else to install. Unsigned: first launch needs right-click → Open, or `xattr -dr com.apple.quarantine /Applications/Signal.app`. |
| `Signal_*_amd64.deb` | Debian/Ubuntu. Depends on `libmpv2 \| libmpv1`, installed automatically by `apt install ./Signal_*.deb`. |
| `Signal_*_amd64.AppImage` | Any glibc distro. `chmod +x` and run. libmpv travels inside the image. |

The `signal` CLI is not shipped as a binary yet — build it with
`cargo build -p signal-cli`.

## Changes

<!-- filled in before publishing -->
