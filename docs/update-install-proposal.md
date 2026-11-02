---
type: Proposal
title: 'Installing updates from the app: a design'
description: Moonglow downloading the release for its own package and installing it (GitHub issue 28's large part, the owner's decisions of 2026-11-02) - the signed SHA256SUMS that every install is checked against and the key built into the app, which package each install takes and where it is downloaded to (beside the running file, then renamed), what a failed or interrupted download leaves, a folder that can't be written, Install Now and Later, the restart, the Flatpak and Scoop left to their tools, Options' two switches, and the check's own request unchanged.
tags: [proposal, updates, packaging, security, minisign]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-02T10:38:19Z }
sources:
  - id: deferred
    resource: deferred.md
    title: 'The deferred list: "Installing updates from the app" (The install, starting and crashes), the large entry 7 after 3.0.0'
  - id: owner
    resource: human:august, 2026-11-02
    title: 'The owner''s decisions: download and install (the AppImage in place, the Windows installer run, the macOS app swapped), restart after asking or at the next start, the notes shown with Install Now and Later, Flatpak and Scoop left to their tools; SHA256SUMS signed with minisign, the public key in the app, the secret key kept off GitHub; Options'' Install updates automatically and Include pre-releases, off'
  - id: code
    resource: crates/mg-ui/src/updates.rs, apps/moonglow/src/update_check.rs, apps/moonglow/src/main.rs (relaunch), packaging/ (read 2026-11-02, at 3.2.0)
    title: The update check and the packages this is built on
---

# Installing updates from the app: a design

Moonglow 2.x and 3.x check GitHub for a newer release and link its
page (Help › Check for Updates…, the daily check).[^code] The builder
then downloads the package and installs it by hand. This design installs
the update from the app.[^deferred] The owner decided its course on
2026-11-02.[^owner]

## What is trusted: a signed list of checksums

A release publishes `SHA256SUMS` beside its packages (`sha256sum`'s
lines: a hash, two spaces, a file name) and `SHA256SUMS.minisig`, its
[minisign](https://jedisct1.github.io/minisign/) signature. The owner
signs it on their own machine with a secret key that never reaches
GitHub (`packaging/sign-release.sh`). The public key is built into the
app from `packaging/minisign.pub` (`apps/moonglow/build.rs`). Until that
file is committed, a build carries no key and installing is unavailable:
the window links the release page, as before.

Before installing, the app checks four things, and refuses the package
if any fails:

1. The signature verifies against the built-in key (minisign's
   prehashed mode, `minisign-verify`). A legacy signature is refused.
2. The signature's trusted comment is `Moonglow Toolset <tag>`, the tag
   of the release being installed. The comment is signed with the file,
   so a list signed for an older release can't be passed off as a newer
   one's. (The release's JSON, its version and its download links, is
   not signed. The binding comes from the comment and the versioned file
   names in the list.)
3. The list names the package's file.
4. The downloaded file's SHA-256 is the one in the list.

On a refusal the window says why ("the signature doesn't verify", "the
release isn't signed") and links the release page. A download link that
isn't under the repository's `releases/download/` is refused before
anything is asked. A compromised GitHub account can publish a release
but can't sign it, so no copy of Moonglow installs it.

## Which package each install takes

What installed this copy is read at start (`mg_update::Kind`):

| This copy | Detected by | Takes | Installed by |
| --- | --- | --- | --- |
| AppImage | `$APPIMAGE` names the running image | `Moonglow-<v>-<arch>.AppImage` | the file at `$APPIMAGE` replaced |
| Windows, installed | `unins000.exe` beside `moonglow.exe` | `Moonglow-<v>-windows-x64-setup.exe` | the installer run, silently, into the same folder |
| Windows, the zip | `moonglow.exe` with no uninstaller beside it | `Moonglow-<v>-windows-x64.zip` | its files swapped into the folder |
| macOS app | the program inside `<name>.app/Contents/MacOS/` | `Moonglow-<v>-macos.dmg` | the app swapped for the image's |
| Flatpak, Scoop, Nix | as before (`Installer`) | nothing | their own tool: the window says how, and links the page |
| anything else (a build from source, a distribution's package) | none of the above | nothing | the page link, as before |

Files of a Flatpak, Scoop or Nix install are never written.

## Where the download goes

Each download is written to a file whose name starts with
`.moonglow-update-` and ends in `.part`, in the folder where the
package is used. When it has been checked, the file is renamed, so a
half-written file never takes a package's name.

- **AppImage:** beside the running image (`$APPIMAGE`'s folder), then
  given the image's permissions and renamed over `$APPIMAGE`. A rename
  in one folder is atomic. The running copy keeps reading the file it
  opened, so Moonglow keeps running; the next start runs the new one.
  The file keeps its name (`Moonglow-3.2.0-x86_64.AppImage` then holds
  3.3.0), so links to it (`~/.local/bin/moonglow`, `mg`) still work.
- **Windows installer:** in Moonglow's cache folder (`updates\`); the
  installer is run, not renamed.
- **Windows zip:** beside the program, then unpacked into a folder
  beside it, `.moonglow-update-<v>`.
- **macOS:** the disk image in the cache folder; its app is copied out
  (`hdiutil attach`, `ditto`, `hdiutil detach`) to
  `.moonglow-update-<v>.app` beside the running app.

**A failed or interrupted download** leaves at most a `.part` file.
Moonglow removes it on failure, and removes any it finds at the next
start: there is no resuming, and the next attempt starts over. A file
that fails a check is removed as well, and nothing is renamed.

**A folder that can't be written** (an AppImage in `/opt`, an app in
`/Applications` for a user who isn't an administrator) is found before
anything is downloaded: the `.part` file can't be created. The window
says "Moonglow can't write to `<folder>`: update it as you installed it"
and links the page.

## Install Now and Later

The notice of a newer release (under the toolbar) has **What's New…**,
**Install Now**, **Later**, **Don't remind me about this version** and
**×**. What's New… opens the update window (Help › Check for Updates…),
which shows the release's notes (the release's Markdown) with the same
buttons.

- **Install Now** downloads and checks the package, showing its
  progress. The AppImage is then replaced at once. The window asks
  "Restart Moonglow now?" with **Restart Now** and **Later**.
- **Restart Now** first asks about unsaved work, as Exit does. Moonglow
  then closes, installs what is left to install (the installer, the zip's
  files, the app) and starts again with the module that was open.
- **Later** downloads and checks the package, and installs it as
  Moonglow closes: the next start is the new version. After a crash
  nothing is installed. The staged files are removed at the next start,
  and the notice offers the update again.

Install updates automatically (below) does Later without being asked.

**Restarting** runs the installed program, not this run's copy of it. For
an AppImage that is `$APPIMAGE`, since the running program is inside its
mount, which goes when it exits. On macOS it is `open -n` of the app, and
for the zip the new `moonglow.exe`. The Windows installer starts Moonglow
itself when it is done: its `[Run]` entry runs only when given
`/restart=1`.

**The steps at close**, in order:

- **AppImage (Later):** the staged file's hash is checked again, then it
  is renamed over `$APPIMAGE`.
- **Windows installer:** the installer's hash is checked again, then
  `setup.exe /SILENT /SUPPRESSMSGBOXES /NORESTART /SP- /CLOSEAPPLICATIONS
  /CURRENTUSER|/ALLUSERS /DIR="<folder>" /LOG="<cache>\updates\install.log"
  [/restart=1]` is started and Moonglow exits. `/ALLUSERS` is used for a
  folder under Program Files, and Windows asks for an administrator
  there.
- **Windows zip:** each file the zip holds is moved: the old one into
  `.moonglow-update-old\` beside it, the new one into its place. A
  running program can be moved on Windows, though not overwritten. A
  failed move puts back what was moved, in reverse order. Only the
  zip's files are touched, so a folder the builder keeps other things in
  is safe.
- **macOS:** the app is renamed to `.moonglow-update-old.app`, the new
  one is renamed to the app's name, and the old one is removed. If the
  second rename fails, the first is undone.

The outcome is written to `updates/last-install.txt` in the cache
folder, and the next start puts it in the log, since the window is gone
by then.

## The check's own request

The check's request stays as it is: one `GET` of
`api.github.com/repos/…/releases/latest`, with the same headers
(`updates.rs`, the user agent `Moonglow/<version>`). With **Include
pre-releases** on, it asks `…/releases?per_page=20` instead (the latest
release is never a pre-release) and takes the newest that isn't a draft.
Installing adds the release's `SHA256SUMS` and `SHA256SUMS.minisig`, and
the one package, from `github.com/…/releases/download/<tag>/`, which
redirects to GitHub's file host. The window and Options say so.
Nothing about the builder, their modules or the computer is sent.

## Options › Updates

Beside Check for updates automatically:

- **Install updates automatically** (off): a newer release the check
  finds is downloaded and checked in the background, and installed as
  Moonglow closes. The notice says so and offers Restart Now.
- **Include pre-releases** (off): the check offers a newer pre-release
  too. A pre-release's own identifiers are compared by semver precedence
  (`3.3.0-rc.2` after `3.3.0-rc.1`, both before `3.3.0`).

The owner named Options › General. The two switches are on the Updates
page, next to the automatic check they extend, which General doesn't
hold.

## Tests

- `mg-update`'s unit tests sign with a key made in the test (the
  `minisign` crate, a development dependency only). They cover a good
  list, a tampered package, a tampered list, another key, a legacy
  signature, the wrong trusted comment and a list that doesn't name the
  package. They also cover the sums' lines, the kind of each install from
  its paths, the asset names, the download to `.part` (a server that cuts
  the body short, an answer of 404), a folder that can't be written, the
  AppImage replaced on a scratch copy, the zip's swap and its undo, and
  the macOS and Windows steps as data (programs, arguments and the
  renames' order).
- The UI tests serve a release from a server of their own on 127.0.0.1
  (`mg_update::serve`). Through the real client (`ureq`, plain HTTP
  allowed for the test alone) they cover the notice, the notes, Install
  Now, the checks passing, the scratch AppImage replaced and the restart
  asked; a bad signature refused with the page link; and a pre-release
  seen only with the option.
- `tests/screens.rs` draws the notice and the window with the notes, the
  progress, the restart question and a refusal.

No test or run reaches GitHub. Running the Windows installer and swapping
the macOS app wait on a Windows machine and a Mac: their steps are unit
tests here, and the deferred list says what is to be tried there.

## What the owner does

1. Makes the key once:
   `minisign -G -p packaging/minisign.pub -s ~/.minisign/moonglow.key`
   (with a password). The secret key stays on their machine and is never
   committed.
2. Commits `packaging/minisign.pub`. Builds from then on carry it.
3. Signs each release after the workflow has drafted it and before
   publishing it: `packaging/sign-release.sh v3.3.0` (`packaging/README.md`,
   Making a release).

[^deferred]: The deferred list's entry
[^owner]: The owner's decisions, 2026-11-02
[^code]: The code read at 3.2.0
