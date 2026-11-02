#!/usr/bin/env bash
# Signs a release's packages, so that Moonglow installs them
# (docs/update-install-proposal.md). Run by the owner on their own machine,
# with the secret key that never leaves it, after the release workflow has
# drafted the release and before it is published:
#
#   packaging/sign-release.sh v3.3.0
#
# Downloads the release's packages (Moonglow-*) with gh, writes their
# SHA256SUMS (sha256sum's lines), signs it with minisign, the release's tag
# in the signature's trusted comment ("Moonglow Toolset v3.3.0", which the
# app checks), verifies the signature against packaging/minisign.pub, and
# uploads SHA256SUMS and SHA256SUMS.minisig to the release (replacing any
# there). minisign asks for the key's password.
#
#   MOONGLOW_MINISIGN_KEY  the secret key (default ~/.minisign/moonglow.key)
#   MOONGLOW_MINISIGN_PUB  the public key (default packaging/minisign.pub)
#   MOONGLOW_REPO          the repository (default jadzziaa/moonglow-toolset)
set -euo pipefail

tag=${1:?usage: packaging/sign-release.sh vX.Y.Z}
repo=${MOONGLOW_REPO:-jadzziaa/moonglow-toolset}
key=${MOONGLOW_MINISIGN_KEY:-$HOME/.minisign/moonglow.key}
here=$(cd "$(dirname "$0")" && pwd)
pub=${MOONGLOW_MINISIGN_PUB:-$here/minisign.pub}

[[ $tag == v* ]] || { echo "the tag is v and the version: v3.3.0" >&2; exit 1; }
[[ -f $pub ]] || { echo "no public key at $pub: make the key first (packaging/README.md)" >&2; exit 1; }
[[ -f $key ]] || { echo "no secret key at $key (MOONGLOW_MINISIGN_KEY)" >&2; exit 1; }
command -v minisign > /dev/null || { echo "minisign isn't installed" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

gh release download "$tag" --repo "$repo" --dir "$work" --pattern 'Moonglow-*'
cd "$work"
shopt -s nullglob
files=(Moonglow-*)
(( ${#files[@]} )) || { echo "the release $tag has no packages" >&2; exit 1; }

if command -v sha256sum > /dev/null; then
    sha256sum -- "${files[@]}" > SHA256SUMS
else
    shasum -a 256 -- "${files[@]}" > SHA256SUMS
fi
minisign -S -s "$key" -m SHA256SUMS -t "Moonglow Toolset $tag"
minisign -V -p "$pub" -m SHA256SUMS
gh release upload "$tag" --repo "$repo" --clobber SHA256SUMS SHA256SUMS.minisig
cat SHA256SUMS
echo "Signed $tag: SHA256SUMS and SHA256SUMS.minisig uploaded. Publish the release now."
