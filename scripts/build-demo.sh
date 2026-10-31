#!/usr/bin/env bash
# Build the public demo: the template's sample days as one event with every
# day unlocked, so the demo is fully playable on any date.
#
# Keys are generated in a throwaway repo for each build and never saved: the
# sample content is public, so there is nothing to keep, and nothing keyed
# ends up in git.
#
# Usage: scripts/build-demo.sh [OUT_DIR]   (default: demo-dist)

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
out=$(cd "$root" && mkdir -p "${1:-demo-dist}" && cd "${1:-demo-dist}" && pwd)
advent=${ADVENT:-$root/target/debug/advent}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

"$advent" init "$work/repo" --event sample > "$work/init.out"
release_key=$(grep '^AGE-SECRET-KEY-1' "$work/init.out")
cd "$work/repo"

cat > events/sample/event.yaml <<'YAML'
title: "Advent demo"
timezone: UTC
start: "2026-12-01"
days: 7
unlocked: 7
notices:
  - text: "Demo build: every door is open with a sample puzzle, and their seven words form the demo key. In a live event each door opens on its own date."
story:
  title: "The Longest Night Hunt"
  intro: "A sample calendar with every door open. Each solve gives you one word of the key; collect all seven and the key opens your certificate."
YAML
sed -i.bak 's/^name: .*/name: Advent demo/' brand/brand.yaml && rm brand/brand.yaml.bak

for n in 1 2 3 4 5 6 7; do
  day="events/sample/day0$n"
  mkdir -p "$day"
  cp -R "examples/day0$n/." "$day/"
  "$advent" seal "$day" > /dev/null 2>&1
done
mkdir -p events/sample/final
cp examples/final/final.yaml events/sample/final/
"$advent" keeper seal sample > /dev/null

# Every day must open and the certificate must open with the seven words.
ADVENT_RELEASE_KEY="$release_key" "$advent" verify --release

# advent build only replaces a folder it wrote itself, so a wrong OUT_DIR
# cannot wipe anything.
ADVENT_RELEASE_KEY="$release_key" "$advent" build --out "$out"

for n in 1 2 3 4 5 6 7; do
  test -f "$out/sample/api/day/$n.json" || { echo "day $n missing from the demo" >&2; exit 1; }
done
echo "Demo built into $out"
