#!/usr/bin/env bash
# Build one showcase (showcases/<name>/: brand and one event, in plaintext)
# into OUT_DIR: seal every day and the certificate with throwaway keys,
# verify, and build the site. The keys are discarded: the content is public.
#
# Usage: scripts/build-showcase.sh <name> <OUT_DIR>   (ADVENT_BASE sets the path)

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
name=$1
src="$root/showcases/$name"
test -d "$src" || { echo "no showcase $name" >&2; exit 1; }
out=$(mkdir -p "$2" && cd "$2" && pwd)
advent=${ADVENT:-$root/target/debug/advent}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

event_dir=$(find "$src/events" -mindepth 1 -maxdepth 1 -type d | head -1)
event=$(basename "$event_dir")

"$advent" init "$work/repo" --event "$event" > "$work/init.out"
release_key=$(grep '^AGE-SECRET-KEY-1' "$work/init.out")
cd "$work/repo"
rm -rf brand "events/$event"
cp -R "$src/brand" brand
cp -R "$event_dir" "events/$event"

for day in "events/$event"/day*; do
  "$advent" seal "$day" > /dev/null 2>&1 || { "$advent" seal "$day"; exit 1; }
done
"$advent" keeper seal "$event" > /dev/null
ADVENT_RELEASE_KEY="$release_key" "$advent" verify --release > /dev/null
ADVENT_RELEASE_KEY="$release_key" "$advent" build --out "$out"
echo "Showcase $name built into $out"
