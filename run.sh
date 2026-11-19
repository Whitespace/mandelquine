#!/usr/bin/env bash
# Run the quine forever: each generation's output becomes the next program.
# Ctrl-C to stop. Generation 60 is byte-identical to mandelquine.rb.
set -euo pipefail
cd "$(dirname "$0")"

mkdir -p tmp
cp mandelquine.rb tmp/quine.rb

while ruby tmp/quine.rb > tmp/next.rb; do
  mv tmp/next.rb tmp/quine.rb
  clear
  cat tmp/quine.rb
done
