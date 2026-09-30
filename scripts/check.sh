#!/usr/bin/env bash
# Local checks for xturing (no remote CI):
#   - rustfmt, clippy (warnings as errors) and the test suite.
#
# Usage: scripts/check.sh
set -u
cd "$(dirname "$0")/.." || exit 1

fail=0

if ! cargo fmt --check >/dev/null 2>&1; then
    echo "fmt FAIL (run: cargo fmt)"
    fail=1
else
    echo "fmt: OK"
fi

if ! cargo clippy --all-targets --quiet -- -D warnings; then
    echo "clippy FAIL"
    fail=1
else
    echo "clippy: OK"
fi

if ! cargo test --quiet; then
    echo "test FAIL"
    fail=1
else
    echo "test: OK"
fi

if [ "$fail" -eq 0 ]; then
    echo "xturing: checks OK"
fi
exit "$fail"
