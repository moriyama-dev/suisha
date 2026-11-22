#!/bin/sh
# macOS check of automatic updates: turn them on in the settings file, start
# Suisha so that it registers its launchd agent, then let launchd run the
# agent once and wait for the entry it writes to the history.
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
