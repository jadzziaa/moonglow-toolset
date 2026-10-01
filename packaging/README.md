# Packaging

Moonglow ships as an AppImage and a Flatpak on Linux, an installer on
Windows and a disk image on macOS. Each package holds the GUI (`moonglow`),
the command-line tools (`mg`), the license, the third-party license notices
and the user manual (`docs/manual`); the manual's
[command-line chapter](../docs/manual/11-command-line.md) says how to run
`mg` from each. They are built with the `dist` Cargo
profile (release with thin LTO; function names kept for crash reports).

`.github/workflows/release.yml` builds all three on a `v*` tag and drafts a
GitHub release with them; run by hand, it leaves them as the run's
artifacts.

| Folder | What |
| --- | --- |
| `icons/` | The icon (`moonglow.svg`, authored for Moonglow) and what `render.sh` makes of it: PNGs from 16 to 512 pixels, `moonglow.ico`, `moonglow.icns`. The 256-pixel PNG is also the window's icon, built into the program. |
| `linux/` | The desktop entry, AppStream metadata, the `.mod` MIME type (weaker than tracker music's, so a file's header decides between them) and `build-appimage.sh`. |
| `flatpak/` | The Flatpak manifest, `cargo_sources.py` and `build-flatpak.sh`. |
| `windows/` | The Inno Setup script and `build-installer.ps1`. The executable's icon and version information come from `apps/moonglow/build.rs`. |
| `macos/` | `Info.plist` and `build-app.sh`. |
| `third_party_licenses.py` | Writes `THIRD-PARTY-LICENSES.txt`: every crate the programs are built from, its license, and the license texts. |

The application ID is `io.github.moonglow_toolset.Moonglow`
(`io.github.moonglow-toolset.Moonglow` on macOS, where bundle IDs take no
underscores). It names the desktop entry, the icon, the AppStream component
and the Wayland app ID (`APP_ID` in `apps/moonglow/src/main.rs`); change
them together. It is provisional until the project has a public home, as is
the AppStream metadata's missing homepage (`appstreamcli validate` warns
about it).

## Linux: AppImage

```sh
packaging/linux/build-appimage.sh
```

Needs [appimagetool](https://github.com/AppImage/appimagetool/releases) on
`PATH` or named by `$APPIMAGETOOL` (it fetches the AppImage runtime from
GitHub as it builds); without it the script stops after
assembling `target/dist/AppDir`, which runs as it is
(`target/dist/AppDir/AppRun`). A link to the image named `mg` runs the
command-line tools instead of the GUI. The image links the system's glibc,
libstdc++ and ALSA (`libasound.so.2`), and loads Vulkan, Wayland and X11 at
run time; build it on the oldest distribution it should run on (the
workflow uses Ubuntu 22.04). File dialogs go through the desktop portal
(`xdg-desktop-portal`), as on every current desktop.

## Linux: Flatpak

```sh
packaging/flatpak/build-flatpak.sh
flatpak install --user target/dist/Moonglow-*.flatpak
flatpak run io.github.moonglow_toolset.Moonglow
```

The script lists the crates in `Cargo.lock` as the build's sources
(`flatpak/cargo_sources.py` writes `cargo-sources.json`, not committed),
so the build itself runs offline; the crates come from Cargo's download
cache where they are, else from crates.io. It builds with the Freedesktop
25.08 SDK and its `rust-stable` extension (installed from Flathub for the
user when missing; about 1.6 GB with the runtime) into
`target/flatpak-repo`, and bundles that as
`target/dist/Moonglow-<version>-<arch>.flatpak`, which fetches the runtime
from Flathub when installed.

The sandbox sees the game where Steam installs it (natively or as a
Flatpak) and the game's user folder, `~/.local/share/Neverwinter Nights`.
A game elsewhere (another Steam library, GOG, Beamdog's client) needs its
folder added:

```sh
flatpak override --user --filesystem="/path/to/Neverwinter Nights:ro" io.github.moonglow_toolset.Moonglow
```

Test Module starts the game from inside the sandbox, with the Flatpak
runtime's libraries; if the game does not start there, use the AppImage.

## Windows: installer

```powershell
packaging\windows\build-installer.ps1
```

Needs Rust (MSVC), Python 3 and [Inno Setup 6](https://jrsoftware.org/isinfo.php).
Writes `target\dist\Moonglow-<version>-windows-x64-setup.exe`, which
installs for the current user (or, when chosen, for all users), adds a
Start menu entry and optionally a desktop icon and the `.mod` association.
The programs carry the C runtime (`.cargo/config.toml`), so no Visual C++
Redistributable is needed.

## macOS: app and disk image

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
packaging/macos/build-app.sh
```

Writes `target/dist/Moonglow Toolset.app` (a universal binary for Apple
silicon and Intel, macOS 11 or later) and
`target/dist/Moonglow-<version>-macos.dmg`. The app is signed ad hoc, so
Gatekeeper refuses its first start: the user allows it in System Settings ›
Privacy & Security › Open Anyway (right-click › Open no longer does it
since macOS 15). For a Gatekeeper-clean release, sign
with a Developer ID (`CODESIGN_IDENTITY="Developer ID Application: ..."`)
and notarize the disk image (`xcrun notarytool submit --wait`, then
`xcrun stapler staple`). Opening a module by double-clicking it in the
Finder is not supported yet (the app would start without it): open modules
from the app.
