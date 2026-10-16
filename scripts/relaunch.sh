#!/bin/sh
# Quit the running Kotoba and start the build in target/debug, a few
# seconds from now, and return at once. The wait lets an agent's reply
# go out before the app it is talking about goes away.
#
#   scripts/relaunch.sh [seconds]     default 5
#   scripts/relaunch.sh --installed [seconds]
#
# With --installed what is started is /Applications/Kotoba.app, not the
# build: the end of a release, when target/ has been deleted
# (scripts/release-clean.sh).

cd "$(dirname "$0")/.." || exit 1
installed=""
[ "$1" = --installed ] && { installed=1; shift; }
delay="${1:-5}"

DELAY="$delay" INSTALLED="$installed" nohup sh -c '
    sleep "$DELAY"
    pkill -x Kotoba
    sleep 2
    [ -z "$INSTALLED" ] && exec scripts/dev-app.sh
    # As dev-app.sh opens the build: the environment a Dock launch has.
    exec env -i \
        HOME="$HOME" USER="$USER" LOGNAME="${LOGNAME:-$USER}" SHELL="${SHELL:-/bin/zsh}" \
        PATH=/usr/bin:/bin:/usr/sbin:/sbin \
        TMPDIR="$(getconf DARWIN_USER_TEMP_DIR)" \
        /usr/bin/open -n /Applications/Kotoba.app
' >/dev/null 2>&1 &
if [ -n "$installed" ]; then
    echo "The installed Kotoba opens ${delay}s from now"
    exit 0
fi
echo "Kotoba relaunches ${delay}s from now"
