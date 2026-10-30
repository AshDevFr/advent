#!/usr/bin/env bash
# Fail when plaintext puzzle content is tracked by git outside the folders that
# hold public examples and test data. .gitignore keeps these files out by
# default, but `git add -f` bypasses it, so CI checks what is actually tracked.

set -euo pipefail

allowed='^(templates|fixtures)/'
content='(^|/)day[^/]*/(puzzle\.md|meta\.yaml|[^/]+\.(png|jpe?g|gif|webp|svg))$|(^|/)final/(final\.yaml|[^/]+\.(png|jpe?g|gif|webp|svg))$'

offenders=$(git ls-files | grep -E "$content" | grep -Ev "$allowed" || true)

if [ -n "$offenders" ]; then
  echo "Plaintext puzzle content is tracked outside templates/ and fixtures/:" >&2
  echo "$offenders" | sed 's/^/  /' >&2
  echo "Remove it with: git rm --cached <file>" >&2
  exit 1
fi

echo "No tracked plaintext content."
