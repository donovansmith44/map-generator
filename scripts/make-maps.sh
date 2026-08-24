#!/usr/bin/env bash
# A bunch of maps for the Bible: the canonical set, rendered through
# the public API into out/maps/ as scalable SVG. Every map is a
# deterministic, content-addressed query — rerunning reproduces the
# same bytes. Needs the demo running (make demo).
set -e
PORT="${PORT:-8090}"
BASE="http://127.0.0.1:$PORT"
OUT="$(cd "$(dirname "$0")/.." && pwd)/out/maps"
mkdir -p "$OUT"

STYLE=$(curl -s "$BASE/api/meta" | grep -o '"id":"[0-9a-f]*","name":"parchment"' | grep -o '[0-9a-f]\{16\}')
[ -n "$STYLE" ] || { echo "demo not running on $BASE (make demo first)"; exit 1; }

render() { # name query-string
  curl -sf "$BASE/api/render?style=$STYLE&width=2400&lod=0&$2" -o "$OUT/$1.svg"
  echo "  $1.svg"
}

echo "cooking into $OUT:"
LEVANT="center=32.6,35.6&zoom=4.5&bible=1"
WIDE="center=33.5,36.5&zoom=8&bible=1"

# The covenant story, map by map (dates are scrub stops, Ussher frame).
render 01-promised-land-num34      "subject=world&year=-1452&$LEVANT"
render 02-twelve-tribes-jos13-19   "subject=world&year=-1400&$LEVANT"
render 03-kingdom-of-saul          "subject=world&year=-1090&$LEVANT"
render 04-solomonic-dominion-1ki4  "subject=world&year=-1000&$WIDE"
render 05-divided-kingdoms-1ki12   "subject=world&year=-900&$LEVANT"
render 06-israel-restored-2ki14    "subject=world&year=-800&$WIDE"
render 07-judah-alone-after-samaria "subject=world&year=-700&$LEVANT"
render 08-yehud-the-return-ezr1    "subject=world&year=-500&$LEVANT"

# Long exposures: covenant history as one still image.
render 09-promise-to-kingdom-exposure "subject=world&year=-1452&to=-1000&$WIDE"
render 10-rise-and-fall-exposure      "subject=world&year=-1095&to=-536&$WIDE"

# The world's context at key moments (all sources, not bible-only).
render 11-world-at-abraham         "subject=world&year=-1900&center=33,38&zoom=18"
render 12-world-at-the-exile       "subject=world&year=-586&center=33,38&zoom=18"
render 13-world-at-messiah         "subject=world&year=-1&center=33,38&zoom=18"

echo "done: $(ls "$OUT" | wc -l) maps"
