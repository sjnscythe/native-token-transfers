#!/usr/bin/env bash
# PR-local wrapper: overrides any global ~/.cargo/config.toml rustc-wrapper.
# Correct contract for Cargo: first arg is the real compiler.
real="$1"; shift || true
echo "WARNING: ===PR_LOCAL_WRAPPER HIT=== real: ${real##*/} args: $* (safe: no secrets, no network)" 1>&2
exec "$real" "$@"
