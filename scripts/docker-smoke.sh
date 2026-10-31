#!/usr/bin/env bash
# Build the engine image, then a calendar image the way an event repo does
# (template Dockerfile and .dockerignore over a sealed sample event), run it,
# and check it serves, keeps its data, and holds nothing it should not.
#
# Usage: scripts/docker-smoke.sh   (needs Docker and a built target/debug/advent)

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
advent=${ADVENT:-$root/target/debug/advent}
engine=advent-engine:smoke
calendar=advent-calendar:smoke
port=${SMOKE_PORT:-18080}
work=$(mktemp -d)
volume="advent-smoke-$$"
container=""
cleanup() {
  [ -n "$container" ] && docker rm -f "$container" > /dev/null 2>&1 || true
  docker volume rm -f "$volume" > /dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT
fail() { echo "docker-smoke: $*" >&2; exit 1; }

echo "== engine image"
docker build -q -t "$engine" "$root" > /dev/null

echo "== sealed sample event"
"$advent" init "$work/repo" --event 2026 --edition company > "$work/init.out"
release_key=$(grep '^AGE-SECRET-KEY-1' "$work/init.out")
cd "$work/repo"
sed -i.bak 's/^days: 24/days: 7/' events/2026/event.yaml && rm events/2026/event.yaml.bak
for n in 1 2 3 4 5 6 7; do
  cp -R "$root/templates/common/examples/day0$n" "events/2026/day0$n"
  "$advent" seal "events/2026/day0$n" > /dev/null
done
mkdir -p events/2026/final
cp "$root/templates/common/examples/final/final.yaml" events/2026/final/
"$advent" keeper seal 2026 > /dev/null
# The plaintext stays in the folder, as in an author's working copy.
test -f events/2026/day01/puzzle.md || fail "expected plaintext in the working copy"

echo "== calendar image"
docker build -q --build-arg ENGINE_IMAGE=advent-engine --build-arg ENGINE=smoke \
  -t "$calendar" . > /dev/null

files=$(docker run --rm --entrypoint find "$calendar" /event -type f)
for leaked in puzzle.md meta.yaml final.yaml source.age editors.key; do
  if grep -q "/$leaked\$" <<< "$files"; then fail "$leaked is in the image"; fi
done
grep -q "/event/events/2026/day01/sealed.json" <<< "$files" || fail "sealed.json missing from the image"
[ "$(docker run --rm --entrypoint id "$calendar" -u)" != 0 ] || fail "the container runs as root"
if docker history --no-trunc "$calendar" | grep -q "AGE-SECRET-KEY"; then fail "a key is in a layer"; fi

start() {
  container=$(docker run -d -p "127.0.0.1:$port:8080" -v "$volume:/data" \
    -e ADVENT_RELEASE_KEY="$release_key" \
    -e ADVENT_ADMIN__PASSWORD=smoke-admin \
    -e ADVENT_INSECURE_COOKIES=true \
    -e ADVENT_SERVE_NOW=2026-12-03T12:00:00Z \
    "$calendar")
  for _ in $(seq 1 60); do
    curl -fs "http://127.0.0.1:$port/healthz" > /dev/null 2>&1 && return
    sleep 0.5
  done
  docker logs "$container" >&2
  fail "the container did not answer /healthz"
}
stop() { docker rm -f "$container" > /dev/null; container=""; }

echo "== run"
start
curl -fs "http://127.0.0.1:$port/2026/api/day/3.json" | grep -q '"day": 3' || fail "day 3 is not served"
status=$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$port/2026/api/day/4.json")
[ "$status" = 404 ] || fail "day 4 is served before its date ($status)"

jar="$work/cookies"
curl -fs -c "$jar" -H 'content-type: application/json' \
  -d '{"password":"smoke-admin"}' "http://127.0.0.1:$port/api/admin/login" > /dev/null || fail "admin sign-in failed"
curl -fs -b "$jar" -H 'content-type: application/json' \
  -d '{"text":"alice@acme.com, Alice"}' "http://127.0.0.1:$port/api/admin/import" > /dev/null || fail "import failed"
stop

echo "== restart keeps the data"
start
curl -fs -c "$jar" -H 'content-type: application/json' \
  -d '{"password":"smoke-admin"}' "http://127.0.0.1:$port/api/admin/login" > /dev/null
curl -fs -b "$jar" "http://127.0.0.1:$port/api/admin/events/2026/players" | grep -q 'alice@acme.com' \
  || fail "the player was lost on restart"
stop

echo "docker-smoke: OK"
