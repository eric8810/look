#!/usr/bin/env bash
# tmux helper library for look E2E acceptance (DESIGN §14.3).
#
# NOTE: This environment has no tmux, so the primary harness is the Python
# pty+pyte runner (test/e2e/run_acceptance.py). This file is provided for
# environments where tmux is available and an tmux-based run is desired.
SESSION="dlook-acc"
BIN="${BIN:-./preview}"
FIX="$(cd "$(dirname "$0")/../fixtures" && pwd)"

tmx_new() {                       # $1=cols $2=rows
  tmux kill-session -t "$SESSION" 2>/dev/null
  tmux new-session -d -s "$SESSION" -x "$1" -y "$2"
  tmux set -g status off
  tmux set -g mouse off
}
tmx_send()  { tmux send-keys -t "$SESSION" "$@"; }
tmx_enter() { tmux send-keys -t "$SESSION" Enter; }
tmx_run()   { tmx_send "$1"; tmx_enter; }
tmx_cap()   { tmux capture-pane -p -e -t "$SESSION"; }
tmx_capp()  { tmux capture-pane -p   -t "$SESSION"; }
tmx_row()   { tmux capture-pane -p -t "$SESSION" | sed -n "${1}p"; }
tmx_kill()  { tmux kill-session -t "$SESSION" 2>/dev/null; }

wait_for() {                      # $1=regex  $2=timeout秒(默认5)
  local pat="$1" t="${2:-5}" i
  for ((i=0;i<t*10;i++)); do
    if tmx_capp | grep -Eq "$pat"; then return 0; fi
    sleep 0.1
  done
  return 1
}
