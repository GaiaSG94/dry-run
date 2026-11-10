#!/usr/bin/env bash
cd "$(dirname "$0")"
exec ./target/release/dryrun-gate serve --port 8787 --web-dir web
