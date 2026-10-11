#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
EXE="$TARGET/release/map-viewer"
PID_FILE="$ROOT/out/demo.pid"
PORT="${PORT:-8090}"
READY_ATTEMPTS=120
WINDOWS=false
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) WINDOWS=true; EXE="$EXE.exe" ;;
esac

owns_pid() {
  local pid="$1"
  [[ "$pid" =~ ^[0-9]+$ ]] && [ "$pid" -gt 1 ] || return 1
  if "$WINDOWS"; then
    DEMO_PID="$pid" DEMO_EXE="$(cygpath -w "$EXE")" powershell -NoProfile -Command \
      'if ((Get-Process -Id $env:DEMO_PID -ErrorAction SilentlyContinue).Path -ne $env:DEMO_EXE) { exit 1 }'
  else
    [ "$(readlink -f "/proc/$pid/exe")" = "$(readlink -f "$EXE")" ]
  fi
}

stop_pid() {
  local pid="$1"
  if "$WINDOWS"; then
    DEMO_PID="$pid" powershell -NoProfile -Command 'Stop-Process -Id $env:DEMO_PID'
  else
    kill "$pid"
  fi
  rm -f "$PID_FILE"
}

case "${1:-start}" in
  start)
    if [ -f "$PID_FILE" ]; then
      pid="$(tr -d '\r\n' < "$PID_FILE")"
      if owns_pid "$pid"; then
        echo "workbench already running: http://127.0.0.1:$PORT/ (PID $pid)"
        exit 0
      fi
      echo "Recorded PID does not belong to this workbench: $pid" >&2
      exit 1
    fi
    if curl --fail --silent --max-time 2 "http://127.0.0.1:$PORT/api/meta" > /dev/null; then
      echo "Port $PORT already serves a workbench without this worktree's PID file" >&2
      exit 1
    fi
    mkdir -p "$ROOT/out"
    if "$WINDOWS"; then
      DEMO_EXE="$(cygpath -w "$EXE")" DEMO_ROOT="$(cygpath -w "$ROOT")" MAP_VIEWER_PORT="$PORT" \
        powershell -NoProfile -Command \
        '(Start-Process -FilePath $env:DEMO_EXE -WorkingDirectory $env:DEMO_ROOT -WindowStyle Hidden -PassThru).Id' > "$PID_FILE"
    else
      [ -x "$EXE" ] || { echo "Build the workbench first: $EXE" >&2; exit 1; }
      MAP_VIEWER_PORT="$PORT" setsid nohup "$EXE" > "$ROOT/out/demo.log" 2>&1 < /dev/null &
      echo "$!" > "$PID_FILE"
    fi
    pid="$(tr -d '\r\n' < "$PID_FILE")"
    for ((attempt=0; attempt<READY_ATTEMPTS; attempt++)); do
      if ! owns_pid "$pid"; then
        echo "Workbench exited before becoming ready; see out/demo.log" >&2
        rm -f "$PID_FILE"
        exit 1
      fi
      if curl --fail --silent --max-time 2 "http://127.0.0.1:$PORT/api/meta" > /dev/null; then
        echo "workbench: http://127.0.0.1:$PORT/ (PID $pid)"
        exit 0
      fi
      sleep 1
    done
    stop_pid "$pid"
    echo "Workbench did not become ready" >&2
    exit 1
    ;;
  stop)
    if [ -f "$PID_FILE" ]; then
      pid="$(tr -d '\r\n' < "$PID_FILE")"
      owns_pid "$pid" || { echo "Refusing to stop a PID that does not belong to this workbench: $pid" >&2; exit 1; }
      stop_pid "$pid"
    fi
    echo "stopped"
    ;;
  *) echo "Usage: $0 start|stop" >&2; exit 2 ;;
esac
