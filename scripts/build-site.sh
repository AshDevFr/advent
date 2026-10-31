#!/usr/bin/env bash
# Assemble everything published at advent-demo.4sh.dev into site/, deployed
# as one Cloudflare Pages upload:
#   /             the documentation (docs/)
#   /demo/        the public demo, built with --base /demo/
#   /northwind/   the Northwind showcase (custom brand, workdays)
#   /christmas/   the 24-day Christmas showcase
#
# Usage: scripts/build-site.sh   (expects `advent` built and the docs installed)

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
site="$root/site"

(cd "$root/docs" && npm run build > /dev/null)

# site/ is this script's own output and nothing else, so it is rebuilt fresh.
rm -rf "$site"
mkdir -p "$site"
cp -R "$root/docs/dist/." "$site/"

ADVENT_BASE=/demo/ "$root/scripts/build-demo.sh" "$site/demo"
for showcase in northwind christmas; do
  ADVENT_BASE="/$showcase/" "$root/scripts/build-showcase.sh" "$showcase" "$site/$showcase"
done

test -f "$site/index.html" || { echo "docs missing from site/" >&2; exit 1; }
test -f "$site/demo/index.html" || { echo "demo missing from site/demo/" >&2; exit 1; }
grep -q '<base href="/demo/"' "$site/demo/index.html" || { echo "demo not built for /demo/" >&2; exit 1; }
echo "Site assembled in $site"
