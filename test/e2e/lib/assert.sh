#!/usr/bin/env bash
# Assertion helpers for look E2E acceptance (DESIGN §14.3).
# Expects $CAP to hold the most recent capture, and $PASS/$FAIL counters.
PASS=${PASS:-0}; FAIL=${FAIL:-0}

say_contains() {                  # $1=desc $2=needle（在 $CAP 中）
  if printf '%s' "$CAP" | grep -Fq -- "$2"; then echo "  ✓ $1"; PASS=$((PASS+1));
  else echo "  ✗ $1"; FAIL=$((FAIL+1)); echo "    缺少: $2"; fi
}
say_matches() {                   # $1=desc $2=regex
  if printf '%s' "$CAP" | grep -Eq -- "$2"; then echo "  ✓ $1"; PASS=$((PASS+1));
  else echo "  ✗ $1"; FAIL=$((FAIL+1)); fi
}
say_not_match() {                 # $1=desc $2=regex（不应出现）
  if printf '%s' "$CAP" | grep -Eq -- "$2"; then echo "  ✗ $1"; FAIL=$((FAIL+1));
  else echo "  ✓ $1"; PASS=$((PASS+1)); fi
}
say_exit() {                      # $1=desc $2=期望退出码
  if printf '%s' "$CAP" | grep -Eq "EXIT:$2($|[^0-9])"; then echo "  ✓ $1"; PASS=$((PASS+1));
  else echo "  ✗ $1 (期望 EXIT:$2)"; FAIL=$((FAIL+1)); fi
}
