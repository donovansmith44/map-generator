#!/usr/bin/env bash
# Start/stop the standing workbench demo as a DETACHED process.
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXE="$ROOT/target/release/map-viewer.exe"

# Windows paths for PowerShell when running under git-bash/MSYS.
EXE_W=$(cygpath -w "$EXE" 2>/dev/null || echo "$EXE")
ROOT_W=$(cygpath -w "$ROOT" 2>/dev/null || echo "$ROOT")

case "${1:-start}" in
  start)
    powershell -NoProfile -Command \
      "Start-Process -FilePath '$EXE_W' -WorkingDirectory '$ROOT_W' -WindowStyle Hidden"
    for i in $(seq 1 120); do
      code=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:${PORT:-8090}/api/meta" || true)
      [ "$code" = "200" ] && break
      sleep 2
    done
    echo "workbench: http://127.0.0.1:${PORT:-8090}/ ($code)"
    ;;
  stop)
    powershell -NoProfile -Command \
      "Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force" 2>/dev/null \
      || pkill -f map-viewer || true
    echo "stopped"
    ;;
esac
