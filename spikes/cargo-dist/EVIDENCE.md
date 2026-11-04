# cargo-dist packaging and release-artifact evidence

Ticket: [Spike cargo-dist packaging and release artifacts](https://github.com/Xaidel/forge/issues/25)
Branch: `spike/cargo-dist` (throwaway, never merged)
Fixture: [`fixture/`](fixture) — package `forge-harness`, binary `forge`, version `0.1.0`
cargo-dist: **0.33.0** (latest stable at the time of the spike, 2026-09-10)
Rust: **1.93.0** (PRD MSRV), edition 2024

## Verdict

`cargo-dist` 0.33.0 can ship the standalone `forge` binary from the MIT-licensed
`forge-harness` package on macOS x64/arm64, Linux x64 GNU, and Windows x64 MSVC with no
Node/npm/Bun runtime, unsigned. Windows arm64 builds natively but is best-effort; see
[Windows arm64](#windows-arm64).

The v1 release shape is:

- GitHub Releases on a `v*` tag, built by a generated `.github/workflows/release.yml`.
- Archives: `forge-harness-<target>.tar.xz` on macOS/Linux, `forge-harness-<target>.zip` on Windows.
- Installers: `forge-harness-installer.sh`, `forge-harness-installer.ps1`,
  `forge-harness-<target>.msi` (Windows), and a Homebrew formula published as `forge.rb`.
- Checksums: one `<archive>.sha256` beside each archive plus a unified `sha256.sum`.
- GitHub artifact attestations through `actions/attest@v4` (`attestations: write`,
  `id-token: write`) for the local build artifacts.
- `cargo install forge-harness` is a separate crates.io publish; cargo-dist does not publish to
  crates.io.
- Unsigned v1: no notarization, no Authenticode. Homebrew and `curl | sh` are the low-friction
  macOS channels; direct browser downloads hit Gatekeeper. Windows may show SmartScreen, and
  Smart App Control can block unsigned binaries outright.

The artifact names are package-based, not binary-based. Because the package must be named
`forge-harness` and the binary `forge`, users see `forge-harness-*` release assets and
`forge-harness-installer.*`. Setting `formula = "forge"` keeps the Homebrew command
`brew install Xaidel/forge/forge`. There is no cargo-dist setting that renames archives or
shell/PowerShell installers to `forge-*`; TDD-0001 should accept the `forge-harness-*` names or add
a release-time rename outside cargo-dist.

## Configuration under test

[`fixture/dist-workspace.toml`](fixture/dist-workspace.toml):

```toml
[workspace]
members = ["cargo:."]

[dist]
cargo-dist-version = "0.33.0"
allow-dirty = ["ci"]
ci = "github"
targets = [
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
  "x86_64-unknown-linux-gnu",
  "x86_64-pc-windows-msvc",
  "aarch64-pc-windows-msvc",
]
installers = ["shell", "powershell", "msi", "homebrew"]
tap = "Xaidel/homebrew-forge"
formula = "forge"
publish-jobs = ["homebrew"]
github-attestations = true
install-path = "CARGO_HOME"
pr-run-mode = "plan"
hosting = "github"
install-updater = false
```

`Cargo.toml` carries the package/bin split and the dist profile:

```toml
[package]
name = "forge-harness"
version = "0.1.0"
edition = "2024"
rust-version = "1.93"
license = "MIT"
description = "..."
repository = "https://github.com/Xaidel/forge"
homepage = "https://github.com/Xaidel/forge"
authors = ["..."]

[[bin]]
name = "forge"
path = "src/main.rs"

[profile.dist]
inherits = "release"
lto = "thin"
```

`allow-dirty = ["ci"]` is evidence-only: the generated release workflow is checked in under
[`generated/release.yml`](generated/release.yml) instead of at the repo root. Production should let
cargo-dist own `.github/workflows/release.yml` and should not set `allow-dirty`.

## Release matrix

The fixture was planned, built, and exercised on GitHub Actions in
[run 41857285306](https://github.com/Xaidel/forge/actions/runs/41857285306). The workflow is
[`.github/workflows/cargo-dist-spike.yml`](../../.github/workflows/cargo-dist-spike.yml); it mirrors
the generated release pipeline but never creates a tag or GitHub Release.

| Target | Runner used | Planned artifact | Build | `forge --version` | Installer evidence |
|---|---|---|---|---|---|
| `aarch64-apple-darwin` | `macos-15` | `.tar.xz` | see run | see run | shell installer; Gatekeeper/`codesign` facts |
| `x86_64-apple-darwin` | `macos-15-intel` | `.tar.xz` | see run | see run | shell installer |
| `x86_64-unknown-linux-gnu` | `ubuntu-24.04` | `.tar.xz` | see run | see run | shell installer against a local HTTP server, with Node shims |
| `x86_64-pc-windows-msvc` | `windows-2025` | `.zip` + `.msi` | see run | see run | PowerShell installer; MSI install/uninstall |
| `aarch64-pc-windows-msvc` | `windows-11-arm` | `.zip` + `.msi` | best-effort; see run | see run | MSI is best-effort; see below |

Local pre-flight on Windows 11 x64 (no WiX installed) confirmed the same artifact list from
`dist plan --tag=v0.1.0` and built `forge-harness-x86_64-pc-windows-msvc.zip`; the MSI leg failed
only because WiX v3 (`candle.exe`) is not on the local PATH. GitHub's Windows x64 image has WiX v3,
so the CI run is the MSI evidence.

### Windows arm64

`aarch64-pc-windows-msvc` compiles and archives natively on the `windows-11-arm` runner. The
generated MSI path is the problem: `windows-11-arm` does not ship the WiX v3 toolchain, while
`windows-2025` does. cargo-dist 0.33.0's MSI template also emits `InstallerVersion='450'`, while
WiX requires 500 for an ARM64 package. Treat the arm64 MSI as best-effort, not a v1 guarantee.

TDD-0001 has three honest choices:

1. Keep `aarch64-pc-windows-msvc` in `targets` but install WiX v3 in `github-build-setup` and
   validate the arm64 MSI before claiming it.
2. Keep arm64 as a zip-only best-effort build and omit arm64 from `targets`, shipping it through a
   separate custom job that passes `-i shell -i powershell -i homebrew` (which excludes MSI).
3. Do not ship arm64 in v1; record it as best-effort and revisit when MXC ships arm64 assets.

The PRD already says Windows arm64 is best-effort (NFR-COMPAT-001, NFR-DIST-001 E3), so option 2
or 3 preserves the v1 guarantee without a failing release job.

## Exact artifact names

From `dist plan --tag=v0.1.0` (fixture version `0.1.0`):

```text
source.tar.gz
source.tar.gz.sha256
forge-harness-installer.sh
forge-harness-installer.ps1
forge.rb
sha256.sum
forge-harness-aarch64-apple-darwin.tar.xz
forge-harness-aarch64-apple-darwin.tar.xz.sha256
forge-harness-x86_64-apple-darwin.tar.xz
forge-harness-x86_64-apple-darwin.tar.xz.sha256
forge-harness-x86_64-unknown-linux-gnu.tar.xz
forge-harness-x86_64-unknown-linux-gnu.tar.xz.sha256
forge-harness-aarch64-pc-windows-msvc.zip
forge-harness-aarch64-pc-windows-msvc.zip.sha256
forge-harness-aarch64-pc-windows-msvc.msi
forge-harness-aarch64-pc-windows-msvc.msi.sha256
forge-harness-x86_64-pc-windows-msvc.zip
forge-harness-x86_64-pc-windows-msvc.zip.sha256
forge-harness-x86_64-pc-windows-msvc.msi
forge-harness-x86_64-pc-windows-msvc.msi.sha256
```

Each archive contains `forge` (`forge.exe` on Windows) plus `README.md` and `LICENSE`. The MSI
embeds the same `forge.exe`. The Homebrew formula is named `forge.rb` because `formula = "forge"`;
without that override it would be `forge-harness.rb`.

## Install paths and behavior

### Shell installer (`forge-harness-installer.sh`)

- Fetches `https://github.com/Xaidel/forge/releases/download/v<version>/<archive>`.
- Default install path `install-path = "CARGO_HOME"`: `$CARGO_HOME/bin`, or `$HOME/.cargo/bin` when
  `CARGO_HOME` is unset. The fixture's generated script contains both.
- Writes `$CARGO_HOME/env` (or `$HOME/.cargo/env`) and adds a line to `$HOME/.profile`; it cannot
  change the current shell's `PATH`.
- Requires `curl` or `wget` and `tar`; no Node/npm/Bun.
- Override for tests or mirrors: `INSTALLER_DOWNLOAD_URL` or the app-specific
  `FORGE_HARNESS_DOWNLOAD_URL`, plus `INSTALLER_BASE_URL` for mirror-aware installers.
- Accepts `--help`, `--version`, `--no-modify-path`, and install-directory flags.

### PowerShell installer (`forge-harness-installer.ps1`)

- Fetches the same release URL and defaults to `$CARGO_HOME\bin`, or `$HOME\.cargo\bin` when
  `CARGO_HOME` is unset.
- Updates `HKCU:\Environment` `Path` as `REG_EXPAND_SZ` and broadcasts `WM_SETTINGCHANGE`; a new
  shell is needed. No admin.
- Requires PowerShell 5.0+ and .NET `System.Net.WebClient`; no Node/npm/Bun.
- Same `INSTALLER_DOWNLOAD_URL` / `FORGE_HARNESS_DOWNLOAD_URL` overrides.

### MSI (`forge-harness-<target>.msi`)

- Per-machine install (`InstallScope='perMachine'`) under
  `%ProgramFiles%\forge-harness\bin\forge.exe`.
- Adds `...\forge-harness\bin` to the system `PATH` by default; the MSI exposes an optional
  "PATH Environment Variable" feature that can be deselected.
- Requires elevation; unsigned, so UAC shows an unknown publisher.
- Requires WiX v3 (`candle.exe`, `light.exe`) on the build machine. GitHub's `windows-2025` image
  includes it; `windows-11-arm` does not. WiX v4 is not supported by cargo-dist 0.33.0.

### Homebrew (`forge.rb`)

The generated formula is a **formula**, not a cask:

```ruby
class Forge < Formula
  desc "..."
  homepage "https://github.com/Xaidel/forge"
  version "0.1.0"
  # macOS arm64/x64 and Linux x64 URLs into the GitHub Release
  license "MIT"
  def install
    bin.install "forge"
    ...
  end
end
```

- Tap: create the `Xaidel/homebrew-forge` repository first, add a `HOMEBREW_TAP_TOKEN` secret with
  `repo` scope, then set `tap`/`publish-jobs` as above.
- Install: `brew install Xaidel/forge/forge`; binary lands in `/opt/homebrew/bin` (Apple Silicon),
  `/usr/local/bin` (Intel macOS), or the Linuxbrew bin directory.
- Homebrew formulae download with `curl` and do not apply the `com.apple.quarantine` xattr, so
  Gatekeeper does not block this channel. That is why Homebrew is the primary macOS channel.
- Homebrew has only one published version per formula; a new release replaces the old one.

### `cargo install forge-harness`

- Installs the `forge` binary into `$CARGO_HOME/bin` from source.
- Requires a Rust toolchain, so it is not the standalone path.
- cargo-dist 0.33.0 does not publish to crates.io: `publish-jobs` only accepts the built-in
  `homebrew` and `npm` jobs or custom jobs. TDD-0001 needs a separate `cargo publish` step (manual
  or a custom `publish-jobs` entry) before `cargo install forge-harness` works.
- Publishing the crate must include the `[[bin]] name = "forge"` target; `cargo install` then
  installs `forge`, not `forge-harness`.

## Checksums and attestations

- `checksum = "sha256"` is the default. Every archive gets `<archive>.sha256` in the
  `sha256sum`-compatible form `<hex> *<archive>`; the `.msi` gets one too.
- cargo-dist also emits a unified `sha256.sum` (since 0.24.0). In the global build job, after local
  artifacts are fetched into `target/distrib`, it is the convenient single file to publish. The
  release checklist should require both the unified file and the per-archive files.
- `github-attestations = true` makes the generated workflow run `actions/attest@v4` in
  `build-local-artifacts`, with job permissions `attestations: write`, `contents: read`, and
  `id-token: write`. The subject is the built local artifact (archive, MSI, checksum).
- Verify with `gh attestation verify <file> --repo Xaidel/forge`.
- GitHub artifact attestations require a public repository or an Enterprise organization; the
  Forge repository is public.
- Unsigned checksums and attestations prove integrity and provenance, not publisher identity; they
  do not satisfy Gatekeeper or SmartScreen.

## Unsigned v1 behavior

### macOS Gatekeeper

- The arm64 binary is ad-hoc signed by the Rust linker, but it is not Developer ID-signed or
  notarized. `spctl --assess` rejects it. The x64 binary may not even be ad-hoc signed.
- A direct browser download carries `com.apple.quarantine`. On macOS 15+ the user cannot
  Control-click to bypass Gatekeeper; they must try to open it, then go to
  **System Settings > Privacy & Security > Open Anyway**. This is the friction NFR-DIST-001 E1
  accepts.
- `curl | sh` and Homebrew formula installs do not set quarantine, so they run without that prompt.
  Recommend them as the macOS paths.
- v1.1 can remove the friction with Developer ID signing + notarization.

### Windows SmartScreen / Smart App Control

- `forge.exe`, the MSI, and the installer script are unsigned: `Get-AuthenticodeSignature` reports
  `NotSigned`.
- A browser-downloaded unsigned exe/MSI can show **Windows protected your PC**; the user chooses
  **More info > Run anyway** unless policy removes that button. Each release hash starts with zero
  reputation; signing would let reputation attach to the publisher.
- Smart App Control on Windows 11 can block unsigned or untrusted code with no per-app exception.
  This is a stronger block than SmartScreen and cannot be fixed by clicking through. TDD-0001 should
  state that Smart App Control may prevent installation and that disabling it is out of scope.
- `irm | iex` runs the PowerShell installer in memory; it does not itself carry Mark-of-the-Web.
- MSI installation requires elevation and shows an unknown publisher in UAC.
- v1.1 can add Authenticode (Azure Artifact Signing is supported by cargo-dist 0.32+ via
  `azure-windows-sign`; ssl.com is supported since 0.14).

### No Node/npm/Bun

- The fixture has no Node, npm, or Bun dependency, and the generated shell/PowerShell installers and
  Homebrew formula contain no `node`, `npm`, `npx`, or `bun` command references.
- The CI run installs with fake `node`/`npm`/`npx`/`bun` shims first on `PATH`; neither the shell
  nor PowerShell installer invokes them. `forge --version` then prints `forge 0.1.0`.
- The installed binary is a native Rust executable; the archives contain the binary, README, and
  LICENSE only.

## CI pipeline facts

The checked-in generated workflow is [`generated/release.yml`](generated/release.yml). It was
produced by `dist generate` from the config above and is the exact file cargo-dist 0.33.0 would
install at `.github/workflows/release.yml`.

- Trigger: tag push matching `'**[0-9]+.[0-9]+.[0-9]+*'` (for example `v0.1.0`) plus
  `pull_request`; `pr-run-mode = "plan"` means PRs only run `dist plan`.
- `plan` runs on `ubuntu-22.04`, installs cargo-dist 0.33.0 with the official installer, caches
  `~/.cargo/bin/dist`, and runs `dist host --steps=create --tag=<tag>` on a tag push.
- `build-local-artifacts` runs per target with `contents: read`, `id-token: write`,
  `attestations: write`; it runs `dist build --tag=<tag> --print=linkage`, attests, and uploads.
- `build-global-artifacts` downloads the local artifacts, runs
  `dist build --tag=<tag> --artifacts=global`, and uploads installers/checksums.
- `host` uploads artifacts and creates the GitHub Release with `gh release create`.
- `publish-homebrew-formula` checks out `Xaidel/homebrew-forge`, downloads the formula, runs
  `brew style --fix`, commits, and pushes with `HOMEBREW_TAP_TOKEN`.
- Default runners are `ubuntu-22.04`, `macos-15-intel`, `macos-15`, and `windows-2022`; the spike
  used `ubuntu-24.04`, `windows-2025`, and `windows-11-arm` as custom runners. `github-custom-runners`
  can pin these in production.
- `dist generate` also creates `wix/main.wxs`; it is checked in at
  [`fixture/wix/main.wxs`](fixture/wix/main.wxs) and should be committed with the release workflow.
- A `dist build --artifacts=local -i shell -i powershell -i homebrew` invocation skips the MSI.
  This is the escape hatch for a zip-only arm64 best-effort job.

## Open decisions for TDD-0001

1. Keep Windows arm64 in the release matrix (install WiX v3 and test the arm64 MSI) or ship it
   zip-only / omit it. The PRD permits best-effort.
2. Accept `forge-harness-*` release asset and installer names, or add a release-time rename. The
   Homebrew formula can still be `forge`.
3. Publish to crates.io with a dedicated `cargo publish` job; cargo-dist will not do it.
4. Use `ubuntu-22.04` (glibc 2.35 floor) or `ubuntu-24.04` (glibc 2.39) for the Linux build. The
   PRD only promises Ubuntu 24.04 LTS, so either is defensible; `ubuntu-22.04` is more forgiving.
5. Use tag-push releases (default) or `dispatch-releases = true` with a manual `dry-run` rehearsal.
6. Decide whether `sha256.sum` or the per-archive `.sha256` files are the canonical checksum
   artifact; publish both.

## Evidence index

- [Fixture config](fixture/dist-workspace.toml)
- [Fixture package](fixture/Cargo.toml)
- [Generated release workflow](generated/release.yml)
- [Generated WiX template](generated/main.wxs)
- [cargo-dist spike workflow](../../.github/workflows/cargo-dist-spike.yml)
- CI run: <https://github.com/Xaidel/forge/actions/runs/41857285306>
- cargo-dist 0.33.0: <https://github.com/axodotdev/cargo-dist/releases/tag/v0.33.0>
- cargo-dist config reference: <https://axodotdev.github.io/cargo-dist/book/reference/config.html>
- cargo-dist artifact URL reference: <https://axodotdev.github.io/cargo-dist/book/reference/artifact-url.html>
- cargo-dist checksums: <https://axodotdev.github.io/cargo-dist/book/artifacts/checksums.html>
- cargo-dist GitHub attestations: <https://axodotdev.github.io/cargo-dist/book/supplychain-security/attestations/github.html>
- cargo-dist MSI installer: <https://axodotdev.github.io/cargo-dist/book/installers/msi.html>
- cargo-dist Homebrew installer: <https://axodotdev.github.io/cargo-dist/book/installers/homebrew.html>
- Apple, "Safely open apps on your Mac": <https://support.apple.com/en-us/102445>
- Microsoft, Smart App Control FAQ: <https://support.microsoft.com/en-us/windows/security/threat-malware-protection/smart-app-control-frequently-asked-questions>
- Microsoft, SmartScreen reputation: <https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation>
