#!/usr/bin/env bash
# Phase acceptance for the static build, end to end with the real binary:
# the sample event built as of 2026-12-03 publishes days 1-3 and nothing
# else, carries no answer or unreleased text, and rebuilds identically.
#
# Usage: scripts/acceptance.sh

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
advent=${ADVENT:-$root/target/debug/advent}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }

"$advent" init "$work/repo" --event 2026 > "$work/init.out"
key=$(grep '^AGE-SECRET-KEY-1' "$work/init.out")
cd "$work/repo"
sed -i.bak 's/^days: 24/days: 7/' events/2026/event.yaml && rm events/2026/event.yaml.bak
for n in 1 2 3 4 5 6 7; do
  mkdir -p "events/2026/day0$n"
  cp -R "examples/day0$n/." "events/2026/day0$n/"
  "$advent" seal "events/2026/day0$n" > /dev/null 2>&1
done
mkdir -p events/2026/final && cp examples/final/final.yaml events/2026/final/
"$advent" keeper seal 2026 > /dev/null

ADVENT_RELEASE_KEY="$key" "$advent" build --until 2026-12-03 --out "$work/one" > /dev/null
ADVENT_RELEASE_KEY="$key" "$advent" build --until 2026-12-03 --out "$work/two" > /dev/null

for n in 1 2 3; do
  test -f "$work/one/2026/api/day/$n.json" || fail "day $n should be published"
done
for n in 4 5 6 7; do
  test ! -e "$work/one/2026/api/day/$n.json" || fail "day $n is published early"
  test ! -e "$work/one/2026/api/day/$n" || fail "day $n assets are published early"
done
echo "ok: days 1-3 published, 4-7 absent"

# Readable files only: ciphertext (locks, certificate) is base64 and the
# minified frontend can contain any short string by chance.
readable=$(cd "$work/one" && find . -type f \( -name '*.json' -o -name '*.css' \) \
  ! -name locks.json ! -name final.json)
for n in 4 5 6 7; do
  title=$(sed -n 's/^title: //p' "examples/day0$n/meta.yaml")
  (cd "$work/one" && grep -lF "$title" $readable) && fail "unreleased title of day $n found"
  while read -r answer; do
    (cd "$work/one" && grep -liwF -- "$answer" $readable) && fail "answer of day $n found"
  done < <(sed -n '/^answers:/,/^[a-z]/s/^  - //p' "examples/day0$n/meta.yaml" | tr -d '"')
done
(cd "$work/one" && grep -rl '"answers"' .) && fail "an answers field was published"
find "$work/one" -name source.age | grep -q . && fail "a source copy was published"
echo "ok: no answer, unreleased title or source in the output"

diff -r "$work/one" "$work/two" > /dev/null || fail "two builds for the same date differ"
echo "ok: rebuild is identical"
echo "Acceptance checks passed."
