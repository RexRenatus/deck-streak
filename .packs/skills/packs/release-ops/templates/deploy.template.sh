#!/bin/sh
# Installs one release beside the others and switches `current` to it with one atomic rename, so
# rolling back is switching to the previous directory. Runs on the host as the deploy user.
# Usage: deploy.sh <tag> <tarball>
set -eu
tag="$1"
tarball="$2"
base="/opt/app"
release="$base/releases/$tag"
printf '%s\n' "$tag" | grep -Eqx 'v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)' || { echo "not a release tag: $tag" >&2; exit 2; }
( cd "$(dirname "$tarball")" && sha256sum -c "$(basename "$tarball").sha256" )
mkdir -p "$release"
tar -xzf "$tarball" -C "$release"
ln -sfn "$release" "$base/current.next"
mv -T "$base/current.next" "$base/current"
sudo systemctl restart app.service
# Keep the five newest releases: the previous tag stays one switch away.
ls -1dt "$base"/releases/*/ | tail -n +6 | xargs -r rm -rf
