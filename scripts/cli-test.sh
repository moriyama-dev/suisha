#!/bin/sh
# Checks the installed `suisha` command from end to end, including a real
# install and removal through the root helper.
# Usage: scripts/cli-test.sh EXPECTED_VERSION
# Meant for a throwaway machine where sudo asks for no password (a CI
# runner): it installs and removes the small apt package "sl".
set -eu

VERSION="$1"

test "$(command -v suisha)" = /usr/bin/suisha
# The command must not need anything a window needs.
if ldd /usr/bin/suisha | grep -E 'webkit|gtk'; then
    echo "The suisha command links against desktop libraries." >&2
    exit 1
fi

suisha version | grep -Fx "suisha $VERSION"
suisha --version | grep -Fx "suisha $VERSION"
suisha help > /dev/null
SUISHA_LANG=ja suisha help | grep -q '使い方'

suisha status
suisha status --json | jq -e --arg v "$VERSION" '.version == $v'
suisha updates
suisha schedule
suisha update --dry-run
suisha update --dry-run --json | jq -e '.dryRun and (.commands | length > 0)'
suisha export | jq -e '.format == "suisha-setup" and (.packages | length > 0)'
suisha report --json | jq -e '.url | startswith("https://github.com/moriyama-dev/suisha/issues/new?")'

# Without a terminal and without --yes, nothing is changed.
if suisha install apt:sl < /dev/null; then
    echo "suisha changed the system without being told to." >&2
    exit 1
fi

suisha install apt:sl --yes
dpkg-query -W -f '${db:Status-Abbrev}\n' sl | grep -q '^ii'
suisha list --manager apt | grep -q '^apt:sl '
suisha remove sl --yes
if dpkg-query -W -f '${db:Status-Abbrev}\n' sl 2> /dev/null | grep -q '^ii'; then
    echo "sl is still installed." >&2
    exit 1
fi
suisha history --json | jq -e '.[0].action == "remove" and .[0].success and .[1].action == "install" and .[1].success'

# The base system is not in the list, so it cannot be removed by accident.
if suisha remove apt:bash --yes; then
    echo "suisha agreed to remove bash." >&2
    exit 1
fi
if suisha frobnicate 2> /dev/null; then
    echo "suisha accepted an unknown command." >&2
    exit 1
fi
echo "The suisha command works."
