#!/usr/bin/env bash
out="$1"; out="${out:-test/fixtures/large.txt}"
mkdir -p "$(dirname "$out")"
: > "$out"
for ((i=0;i<2000;i++)); do
  if (( i % 10 == 0 )); then printf 'MARKER:%04d\n' "$i" >> "$out";
  else printf 'LINE_%04d\n' "$i" >> "$out"; fi
done
