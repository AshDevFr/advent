#!/usr/bin/env bash
# Assemble everything published at advent-demo.4sh.dev into site/, deployed
# as one Cloudflare Pages upload:
#   /             the documentation (docs/)
#   /demo/        the public demo, built with --base /demo/
#   /northwind/   the Northwind showcase (custom brand, workdays)
#   /christmas/   the 24-day Christmas showcase
#
# With ADVENT_SCREENSHOTS=1, the docs' screenshots are captured from the
# freshly built demo and showcases before the docs are built (needs
# Playwright's Chromium: `npx --prefix docs playwright install chromium`).
#
# Usage: scripts/build-site.sh   (expects `advent` built and the docs installed)

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
site="$root/site"

# site/ is this script's own output and nothing else, so it is rebuilt fresh.
rm -rf "$site"
mkdir -p "$site"

ADVENT_BASE=/demo/ "$root/scripts/build-demo.sh" "$site/demo"
for showcase in northwind christmas; do
  ADVENT_BASE="/$showcase/" "$root/scripts/build-showcase.sh" "$showcase" "$site/$showcase"
done

if [ "${ADVENT_SCREENSHOTS:-0}" = "1" ]; then
  (cd "$root/docs" && node scripts/screenshots.mjs "$site" public/screenshots)
fi

(cd "$root/docs" && npm run build > /dev/null)
cp -R "$root/docs/dist/." "$site/"

test -f "$site/index.html" || { echo "docs missing from site/" >&2; exit 1; }
for path in demo northwind christmas; do
  test -f "$site/$path/index.html" || { echo "$path missing from site/" >&2; exit 1; }
  grep -q "<base href=\"/$path/\"" "$site/$path/index.html" || { echo "$path not built for /$path/" >&2; exit 1; }
done
echo "Site assembled in $site"
