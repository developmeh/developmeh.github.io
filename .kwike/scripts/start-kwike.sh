#!/usr/bin/env bash
# Start kwike daemon and consumers for developmeh.com
# Usage: start-kwike.sh [--daemon-only] [--consumers-only]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KWIKE_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$KWIKE_DIR")"

DAEMON_SOCKET="${KWIKE_DIR}/daemon.sock"
PID_DIR="${KWIKE_DIR}/pids"

mkdir -p "$PID_DIR"
mkdir -p "${KWIKE_DIR}/locks"

# Check if a process is running from its PID file
is_running() {
  local pidfile="$1"
  if [[ -f "$pidfile" ]]; then
    local pid
    pid=$(cat "$pidfile")
    if kill -0 "$pid" 2>/dev/null; then
      return 0
    fi
  fi
  return 1
}

start_daemon() {
  local pidfile="${PID_DIR}/daemon.pid"

  if is_running "$pidfile"; then
    echo "Daemon already running (PID $(cat "$pidfile"))"
    return 0
  fi

  # Clean up stale socket if process is dead
  if [[ -S "$DAEMON_SOCKET" ]]; then
    echo "Removing stale daemon socket..."
    rm "$DAEMON_SOCKET"
  fi
  [[ -f "$pidfile" ]] && rm "$pidfile"

  echo "Starting kwike daemon..."
  kwike daemon --socket "$DAEMON_SOCKET" &
  DAEMON_PID=$!
  echo "$DAEMON_PID" > "$pidfile"
  echo "Daemon started with PID $DAEMON_PID"

  # Wait for socket to be ready
  for i in {1..30}; do
    if [[ -S "$DAEMON_SOCKET" ]]; then
      echo "Daemon socket ready"
      return 0
    fi
    sleep 0.1
  done

  echo "WARNING: Daemon socket not ready after 3 seconds"
}

start_consumer() {
  local name="$1"
  local config_path="${KWIKE_DIR}/agents/${name}/consumer.yaml"
  local pidfile="${PID_DIR}/${name}.pid"

  if is_running "$pidfile"; then
    echo "${name} already running (PID $(cat "$pidfile"))"
    return 0
  fi

  # Clean up stale PID file
  [[ -f "$pidfile" ]] && rm "$pidfile"

  echo "Starting ${name} consumer..."
  # Run from repo root so relative paths work
  cd "$REPO_ROOT"
  kwike consume --config "$config_path" &
  local pid=$!
  echo "$pid" > "$pidfile"
  echo "${name} started with PID $pid"
}

start_consumers() {
  start_consumer "doc-organizer"
  start_consumer "commit-agent"
}

stop_all() {
  echo "Stopping kwike processes..."

  for pidfile in "${PID_DIR}"/*.pid; do
    if [[ -f "$pidfile" ]]; then
      PID=$(cat "$pidfile")
      NAME=$(basename "$pidfile" .pid)
      if kill -0 "$PID" 2>/dev/null; then
        echo "Stopping $NAME (PID $PID)..."
        kill "$PID" || true
      fi
      rm "$pidfile"
    fi
  done

  if [[ -S "$DAEMON_SOCKET" ]]; then
    rm "$DAEMON_SOCKET"
  fi

  echo "All kwike processes stopped"
}

status() {
  echo "Kwike Status:"
  echo "============="

  if [[ -S "$DAEMON_SOCKET" ]]; then
    echo "Daemon: RUNNING (socket: $DAEMON_SOCKET)"
  else
    echo "Daemon: STOPPED"
  fi

  for pidfile in "${PID_DIR}"/*.pid; do
    if [[ -f "$pidfile" ]]; then
      PID=$(cat "$pidfile")
      NAME=$(basename "$pidfile" .pid)
      if kill -0 "$PID" 2>/dev/null; then
        echo "$NAME: RUNNING (PID $PID)"
      else
        echo "$NAME: DEAD (stale PID file)"
      fi
    fi
  done

  LOCK_COUNT=$(find "${KWIKE_DIR}/locks" -name "*.lock" -type f 2>/dev/null | wc -l)
  echo ""
  echo "Active locks: $LOCK_COUNT"
}

case "${1:-start}" in
  start)
    start_daemon
    sleep 1
    start_consumers
    echo ""
    echo "Kwike is running. Use '$0 status' to check status."
    ;;
  daemon-only|--daemon-only)
    start_daemon
    ;;
  consumers-only|--consumers-only)
    start_consumers
    ;;
  stop)
    stop_all
    ;;
  restart)
    stop_all
    sleep 1
    start_daemon
    sleep 1
    start_consumers
    ;;
  status)
    status
    ;;
  *)
    echo "Usage: $0 {start|stop|restart|status|daemon-only|consumers-only}"
    exit 1
    ;;
esac
