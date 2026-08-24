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

# The wider Word: nations, vision, journeys, the NT.
render 14-table-of-nations-gen10   "subject=world&year=-2200&center=33,36&zoom=22&bible=1"
render 15-exodus-journeys-num33    "subject=world&year=-1470&center=30.2,34.3&zoom=5&bible=1"
render 16-canaan-before-conquest   "subject=world&year=-1460&$LEVANT"
render 17-land-in-vision-ezk47     "subject=world&year=-570&$LEVANT"
render 18-tetrarchies-luk3         "subject=world&year=30&center=32.3,35.4&zoom=2.5&bible=1"
render 19-pauls-journeys-acts      "subject=world&year=64&center=37.5,28&zoom=12&bible=1"

# The world's context at key moments (all sources, not bible-only).
render 11-world-at-abraham         "subject=world&year=-1900&center=33,38&zoom=18"
render 12-world-at-the-exile       "subject=world&year=-586&center=33,38&zoom=18"
render 13-world-at-messiah         "subject=world&year=-1&center=33,38&zoom=18"

echo "done: $(ls "$OUT" | wc -l) maps"
