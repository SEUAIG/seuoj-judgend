#!/usr/bin/env bash
set -euo pipefail
# Run only in an isolated checkout/container, never alongside live problem data.
if [ "$(printenv SEUOJ_DISPOSABLE_TESTS || true)" != 1 ]; then
 echo 'Set SEUOJ_DISPOSABLE_TESTS=1 only in a disposable test checkout.' >&2; exit 1
fi
cd "$(dirname "$0")/.."
if [ -e assets/problems ]; then
 echo 'Refusing to overwrite existing problem data.' >&2; exit 1
fi
mkdir -p assets
cp -a tests/fixtures/problems assets/problems
cp -a assets/problems/test01 assets/problems/test02
chmod +x assets/problems/test01/data/checker assets/problems/test02/data/checker
