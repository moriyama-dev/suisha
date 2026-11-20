#!/bin/sh
# Launch test for a built Suisha binary: print what it can see, start the
# window, and check that it is still running after a while.
# Usage: scripts/launch-test.sh PATH/TO/suisha
# On Linux, run it inside `xvfb-run -a dbus-run-session -- ...`.
set -eu

APP="$1"
LOG="${RUNNER_TEMP:-/tmp}/suisha-launch.log"

"$APP" --version
"$APP" --self-check

"$APP" > "$LOG" 2>&1 &
pid=$!
sleep 20

if ! kill -0 "$pid" 2> /dev/null; then
    echo "Suisha exited during the launch test:" >&2
    cat "$LOG" >&2
    exit 1
fi

if command -v xwininfo > /dev/null 2>&1; then
    if ! xwininfo -root -tree | grep -q '"Suisha"'; then
        echo "Suisha is running but its window was not found." >&2
        xwininfo -root -tree >&2
        kill "$pid"
        exit 1
    fi
fi

kill "$pid"
wait "$pid" 2> /dev/null || true
cat "$LOG"
echo "Suisha stayed up for 20 seconds."
