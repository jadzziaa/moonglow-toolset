# Packaging

Moonglow ships as an AppImage and a Flatpak on Linux, an installer on
Windows and a disk image on macOS. Each package holds the GUI (`moonglow`),
the command-line tools (`mg`), the license, the third-party license notices
and the user manual (`docs/manual`). The manual's
[command-line chapter](../docs/manual/11-command-line.md) says how to run
`mg` from each. Packages are built with the `dist` Cargo profile: release
with thin LTO, keeping function names for crash reports, and `mg` with its
default features: without `nui-preview` it has no `mg nui render` or
`mg nui layout` (they draw with the GUI's library, on the GPU).

`.github/workflows/release.yml` builds all three on a `v*` tag and drafts a
GitHub release with them. Run by hand, it leaves them as the run's artifacts.

## Making a release

The release is the version of the two programs, `mg` and `moonglow`:
the `version` line of `[workspace.package]` in the root `Cargo.toml`,
which they take (`version.workspace`) and the package scripts read. The
libraries keep `version = "0.0.0"` and are not touched, so a release
rebuilds the two programs alone; what the libraries say of the release
(an export's stamp, an NWSync manifest, the About box, the update check,
the debug log, the compiled shaders' cache) they read from the program,
which sets it as it starts (`mg_core::release`). A release:

1. Changes that one line: `sed -i 's/^version = "2.8.0"/version = "2.9.0"/' Cargo.toml`.
2. Brings `Cargo.lock` along (`cargo metadata --format-version 1 > /dev/null`):
   only the `mg` and `moonglow` entries change.
3. Adds a `<release version="…" date="…">` entry at the top of
   `<releases>` in `linux/io.github.moonglow_toolset.Moonglow.metainfo.xml`.
4. Is committed ("Version 2.9.0") and tagged `v2.9.0`; the tag builds the
   packages and drafts the GitHub release.
5. Is signed, on the owner's machine, before it is published:
   `packaging/sign-release.sh v2.9.0` ([Signing a release](#signing-a-release)).
   A release published unsigned is offered by the app with its page
   only: nothing installs it.
6. Once the release is published, the Scoop manifest follows ([Scoop](#scoop)).

| Folder | What |
| --- | --- |
| `icons/` | The icon (`moonglow.svg`, authored for Moonglow) and what `render.sh` makes of it: PNGs from 16 to 512 pixels, `moonglow.ico`, `moonglow.icns`. The 256-pixel PNG is also the window's icon, built into the program. |
| `linux/` | The desktop entry, AppStream metadata, the `.mod` MIME type (weaker than tracker music's, so a file's header decides between them) and `build-appimage.sh`. |
| `flatpak/` | The Flatpak manifest, `cargo_sources.py` and `build-flatpak.sh`. |
| `windows/` | The Inno Setup script, `build-installer.ps1`, and `dxc.ps1` with `dxc/` (the pinned DirectX Shader Compiler: [DXC](#dxc-direct3d-12s-shader-compiler)). The executable's icon and version information come from `apps/moonglow/build.rs`. |
| `macos/` | `Info.plist` and `build-app.sh`. |
| `sign-release.sh` | Signs a release's packages' checksums with the owner's minisign key and uploads them ([Signing a release](#signing-a-release)); `minisign.pub` beside it, once made, is the public key built into the app. |
| `third_party_licenses.py` | Writes `THIRD-PARTY-LICENSES.txt`: every crate the programs are built from, its license, and the license texts (on Windows, DXC's too: `--dxc`). |

The application ID is `io.github.moonglow_toolset.Moonglow`
(`io.github.moonglow-toolset.Moonglow` on macOS, where bundle IDs take no
underscores). It names the desktop entry, the icon, the AppStream component
and the Wayland app ID (`APP_ID` in `apps/moonglow/src/main.rs`); change
them together. The ID is provisional until the project has a public home, as
is the AppStream metadata's missing homepage (`appstreamcli validate` warns
about it).

## Signing a release

Moonglow installs an update only when the release's `SHA256SUMS` (the
packages' SHA-256 checksums, `sha256sum`'s lines) is signed with the
project's [minisign](https://jedisct1.github.io/minisign/) key, its
trusted comment names the release's tag (`Moonglow Toolset v2.9.0`), and
the downloaded package's checksum is the one in it
(`docs/update-install-proposal.md`; `crates/mg-update`). The public key
is built into the app from `packaging/minisign.pub`
(`apps/moonglow/build.rs`); a build without that file carries no key and
installs nothing, linking the release page as before. The secret key is
the owner's, kept off GitHub and out of the repository (`.gitignore`
refuses `packaging/*.key` and `minisign.key`), and used only on the
owner's machine.

Once, to make the key (minisign asks for a password, which protects the
secret key; keep both the key file and the password backed up: a lost
key means a new public key, and copies built with the old one can't
install updates signed with the new):

```sh
minisign -G -p packaging/minisign.pub -s ~/.minisign/moonglow.key
git add packaging/minisign.pub
git commit -m "The update key"
```

For each release, after the tag's workflow has drafted it and before it
is published:

```sh
packaging/sign-release.sh v2.9.0
```

It downloads the draft's packages (`Moonglow-*`) with `gh` into a
scratch folder, writes `SHA256SUMS`, signs it (`minisign -S -s
~/.minisign/moonglow.key -m SHA256SUMS -t "Moonglow Toolset v2.9.0"`,
asking for the password), verifies the signature against
`packaging/minisign.pub`, and uploads `SHA256SUMS` and
`SHA256SUMS.minisig` to the release (`gh release upload --clobber`).
Then publish the release. A package added or rebuilt after signing
needs the script run again. `MOONGLOW_MINISIGN_KEY` names another
secret key file. `crates/mg-update/tests/sign_release.rs` runs the
script with `gh` and `minisign` stood in for, so the file it writes is
held to what the app reads.

## Linux: AppImage

```sh
packaging/linux/build-appimage.sh
```

Needs [appimagetool](https://github.com/AppImage/appimagetool/releases) on
`PATH` or named by `$APPIMAGETOOL`; it fetches the AppImage runtime from
GitHub as it builds. Without it, the script stops after assembling
`target/dist/AppDir`, which runs as it is (`target/dist/AppDir/AppRun`).

A link to the image named `mg` runs the command-line tools instead of the
GUI. The image links the system's glibc, libstdc++ and ALSA
(`libasound.so.2`) and loads Vulkan, Wayland and X11 at run time. Build it
on the oldest distribution it should run on (the workflow uses Ubuntu
22.04). File dialogs go through the desktop portal (`xdg-desktop-portal`),
as on every current desktop.

## Linux: Flatpak

```sh
packaging/flatpak/build-flatpak.sh
flatpak install --user target/dist/Moonglow-*.flatpak
flatpak run io.github.moonglow_toolset.Moonglow
```

The script lists the crates in `Cargo.lock` as the build's sources
(`flatpak/cargo_sources.py` writes `cargo-sources.json`, not committed),
so the build itself runs offline. The crates come from Cargo's download
cache where they are, else from crates.io. The build uses the Freedesktop
25.08 SDK and its `rust-stable` extension, installed from Flathub for the
user when missing (about 1.6 GB with the runtime). It builds into
`target/flatpak-repo` and bundles that as
`target/dist/Moonglow-<version>-<arch>.flatpak`, which fetches the runtime
from Flathub when installed.

The sandbox sees the game where Steam installs it (natively or as a
Flatpak) and the game's user folder, `~/.local/share/Neverwinter Nights`.
A game elsewhere (another Steam library, GOG, Beamdog's client) needs its
folder added:

```sh
flatpak override --user --filesystem="/path/to/Neverwinter Nights:ro" io.github.moonglow_toolset.Moonglow
```

Test Module starts the game inside the sandbox, with the Flatpak runtime's
libraries. If the game does not start there, use the AppImage.

The window's accessibility tree (AccessKit, for screen readers and
desktop agents) goes over the AT-SPI bus, which Flatpak proxies on its
own; AccessKit also reads whether a screen reader is on from the session
bus's `org.a11y.Bus`, which the manifest's `--talk-name=org.a11y.Bus`
allows. Without it the tree is never offered in the Flatpak.

## Windows: installer

```powershell
packaging\windows\build-installer.ps1
```

Needs Rust (MSVC), Python 3, [Inno Setup 6](https://jrsoftware.org/isinfo.php)
and the network the first time (it downloads [DXC](#dxc-direct3d-12s-shader-compiler)).
Writes `target\dist\Moonglow-<version>-windows-x64-setup.exe`, and the same
files as `Moonglow-<version>-windows-x64.zip` (to unpack anywhere and run). It installs
for the current user (or for all users, if chosen) and adds a Start menu
entry and, optionally, a desktop icon and the `.mod` association.
The programs carry the C runtime (`.cargo/config.toml`), so no Visual C++
Redistributable is needed.

### DXC: Direct3D 12's shader compiler

Both the installer and the zip put Microsoft's DirectX Shader Compiler,
`dxcompiler.dll` (x64), beside `moonglow.exe` and `mg.exe`. Direct3D 12
compiles shaders with the one beside the program (`crates/mg-render/src/dx12.rs`;
the debug log's `graphics:` line says which), and with Windows' FXC where
there is none, which can't compile the game's normal-mapped lit shaders
(`fslit_nm`, `fslit_sm_nm`: a texture read with derivatives in a loop
of a count that varies by pixel). `windows/dxc.ps1` fetches it as the
installer is built:
the release pinned there (`v1.9.2609`, `dxc_2026_09_29.zip`, SHA-256
`ad31b1fc8443175d204f77a611fdb3ef2ec42759bdc2f1167368de24a4a7e7f1`),
downloaded from GitHub into `target\dxc\<tag>`, its hash checked, and
`bin\x64\dxcompiler.dll` copied out. `dxil.dll` is neither needed (since
1.8.2505 `dxcompiler.dll` validates and signs what it compiles on its own) nor
shipped. CI's Windows job puts the same DLL beside the test binaries
(`target\debug\deps`) and sets `MOONGLOW_REQUIRE_DXC=1`, so that
mg-render's compiler check
(`a_texture_read_in_a_loop_of_a_varying_count_compiles_and_draws`) fails
unless Direct3D 12 compiled with it and drew such a loop right.

DXC is under the University of Illinois/NCSA Open Source License
(LLVM's), with Microsoft's own code under MIT: both permissive, and fine
to ship with a GPL-3.0 program. `THIRD-PARTY-LICENSES.txt` has both texts,
from the zip, and the repository's `ThirdPartyNotices.txt` at the tag
(`windows/dxc/`, for what DXC incorporates; `third_party_licenses.py
--dxc`). The zip also holds `LICENSE-MS.txt`, a proprietary Microsoft
license that names no file. It is read here as covering what is closed in
the zip (`dxil.dll`, the validator, built apart from the open source the
repository's README puts under NCSA), which isn't shipped. That reading
is an inference, not a statement in the zip: the owner was asked to
confirm it.

A newer DXC: change the tag, the zip's name and its hash in `windows/dxc.ps1`
(the hash is the release asset's digest on GitHub; check it against the
file), replace `windows/dxc/ThirdPartyNotices.txt` with the new tag's, and
run the corpus HLSL test through it (`MOONGLOW_DXC`, `tools/dx12/dxc.sh`)
and CI's Windows job. wgpu 30 needs 1.8.2502 or later.

### Scoop

The repository is its own [Scoop](https://scoop.sh) bucket: `bucket/moonglow.json`
installs the release's zip (the installer's files; Scoop's unpacker can't
read the Inno Setup 6.7 installer), puts `mg` on the PATH and adds a Start
menu entry.

```powershell
scoop bucket add moonglow https://github.com/jadzziaa/moonglow-toolset
scoop install moonglow
scoop update moonglow
```

After publishing a release, point the manifest at it and commit it:

```sh
packaging/scoop/update-manifest.sh <version>
```

It takes the zip's SHA-256 from the release on GitHub. Until the
manifest is committed, Scoop users stay on the release before.

### Updates in package-managed installs

The GUI asks GitHub for the latest release once a day at most (Options ›
Updates) and installs it from the app where it updates itself
([Signing a release](#signing-a-release); the manual's main-window
chapter). Where a package manager installed it, it doesn't: the Flatpak
(`FLATPAK_ID` set or `/.flatpak-info` there), Scoop (the program under a
`scoop\apps\` folder) and Nix (under `/nix/store`) are told apart at
start (`mg_update::Kind`), Help › Check for Updates… says to update
through them, and their files are never written. The AppImage (by
`$APPIMAGE`), the installer (its `unins000.exe` beside `moonglow.exe`),
the zip (no uninstaller) and the macOS app (the program inside
`*.app/Contents/MacOS/`) check and install. The Flatpak isn't given the
network (`finish-args`), and its Check for Updates… asks nothing: it
says to update with `flatpak update`. A new way of installing that
updates Moonglow itself belongs in that list.

How each is installed (the steps are `mg_update::install::plan`'s):

- **AppImage:** downloaded beside the image, checked, given its
  permissions and renamed over `$APPIMAGE` (at once with Install Now, as
  Moonglow closes with Later). Moonglow restarts as `$APPIMAGE`.
- **Windows installer:** downloaded to Moonglow's cache folder
  (`updates\`), checked, and run as Moonglow closes:
  `setup.exe /SILENT /SUPPRESSMSGBOXES /NORESTART /SP- /CLOSEAPPLICATIONS
  /CURRENTUSER|/ALLUSERS /DIR=<folder> /LOG=<cache>\updates\install.log
  [/MOONGLOWRESTART=1]`. `/MOONGLOWRESTART=1` runs `moonglow.iss`'s second `[Run]` entry,
  which starts Moonglow again.
- **Windows zip:** unpacked into `.moonglow-update-<version>` beside the
  program, then each of its files moved in, the old one moved into
  `.moonglow-update-old` (removed at the next start), undone if a move
  fails.
- **macOS:** the disk image's app copied out (`hdiutil attach -nobrowse
  -readonly`, `ditto`, `hdiutil detach`) to `.moonglow-update-<version>.app`
  beside the app, which is moved aside and replaced as Moonglow closes,
  then started with `open -n`.

The Windows installer and zip, and the macOS swap, are unit tests of
their steps here; they wait to be tried on Windows and a Mac
(`docs/deferred.md`).

## macOS: app and disk image

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
packaging/macos/build-app.sh
```

Writes `target/dist/Moonglow Toolset.app` (a universal binary for Apple
silicon and Intel, macOS 11 or later) and
`target/dist/Moonglow-<version>-macos.dmg`. The app is signed ad hoc, so
Gatekeeper refuses its first start. The user allows it in System Settings ›
Privacy & Security › Open Anyway (since macOS 15, right-click › Open no
longer does it). For a Gatekeeper-clean release, sign with a Developer ID
(`CODESIGN_IDENTITY="Developer ID Application: ..."`) and notarize the disk
image (`xcrun notarytool submit --wait`, then `xcrun stapler staple`).
Opening a module by double-clicking it in the Finder is not supported yet
(the app would start without it); open modules from the app.
