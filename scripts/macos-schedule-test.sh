#!/bin/sh
# macOS check of automatic updates: turn them on in the settings file, start
# Suisha so that it registers its launchd agent, then let launchd run the
# agent once and wait for the entry it writes to the history. Then the same
# for the check at startup, which is on without being turned on.
# Usage: scripts/macos-schedule-test.sh PATH/TO/Suisha.app/Contents/MacOS/suisha
set -eu

# The agent stores the absolute path.
APP="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
DATA="$HOME/Library/Application Support/suisha"
LABEL="dev.moriyama.suisha.auto-update"
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
DOMAIN="gui/$(id -u)"
LOG="$HOME/Library/Logs/Suisha/auto-update.log"

mkdir -p "$DATA"
cat > "$DATA/settings.json" <<'JSON'
{
  "autoUpdate": {
    "enabled": true,
    "frequency": "daily",
    "hour": 3,
    "minute": 30,
    "managers": ["brew-cask"],
    "notify": true
  },
  "language": "en"
}
JSON
rm -f "$DATA/history.jsonl"

# Starting the app keeps the agent in line with the settings.
"$APP" > /dev/null 2>&1 &
pid=$!
sleep 15
kill "$pid"
wait "$pid" 2> /dev/null || true

plutil -lint "$PLIST"
cat "$PLIST"
grep -F "<string>$APP</string>" "$PLIST"
launchctl print "$DOMAIN/$LABEL" | head -n 30

# Run it now instead of waiting for 03:30.
launchctl kickstart "$DOMAIN/$LABEL"
i=0
until grep -q '"trigger":"scheduled"' "$DATA/history.jsonl" 2> /dev/null; do
    i=$((i + 1))
    if [ "$i" -gt 120 ]; then
        echo "The scheduled run wrote no history within 20 minutes." >&2
        cat "$LOG" >&2 || true
        exit 1
    fi
    sleep 10
done

echo "History:"
cat "$DATA/history.jsonl"
echo "Log:"
cat "$LOG"

launchctl bootout "$DOMAIN" "$PLIST"
rm -f "$PLIST"
echo "The launchd agent ran Suisha's automatic update."

# The check at startup: the window registered a login agent too. launchd
# ran it when it was loaded, and that run must have done nothing.
STARTUP="dev.moriyama.suisha.startup-check"
STARTUP_PLIST="$HOME/Library/LaunchAgents/$STARTUP.plist"
STARTUP_LOG="$HOME/Library/Logs/Suisha/startup-check.log"
plutil -lint "$STARTUP_PLIST"
grep -F "<string>--startup-check</string>" "$STARTUP_PLIST"
launchctl print "$DOMAIN/$STARTUP" | head -n 30
if [ -e "$DATA/startup-check.json" ]; then
    echo "The check ran when the agent was loaded, not at a login." >&2
    exit 1
fi

# What a login does: run it again, past the moment it was loaded.
rm -f "$DATA/startup-agent-loaded"
launchctl kickstart "$DOMAIN/$STARTUP"
i=0
until [ -e "$DATA/startup-check.json" ]; do
    i=$((i + 1))
    if [ "$i" -gt 120 ]; then
        echo "The check at startup wrote nothing within 20 minutes." >&2
        cat "$STARTUP_LOG" >&2 || true
        exit 1
    fi
    sleep 10
done
echo "Check at startup:"
cat "$DATA/startup-check.json"
echo
cat "$STARTUP_LOG" || true

launchctl bootout "$DOMAIN" "$STARTUP_PLIST"
rm -f "$STARTUP_PLIST"
echo "The login agent ran Suisha's check at startup."
