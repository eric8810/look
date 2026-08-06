#!/usr/bin/env bash
# Acceptance test runner. tmux is unavailable in this environment, so we use
# a Python pty + pyte harness with equivalent fidelity (real pty, key send,
# screen capture, exit codes). Set BIN to the binary under test.
set -u
BIN="${BIN:-node dist/terminal.js}"
cd "$(dirname "$0")/../.."
python3 test/e2e/run_acceptance.py
