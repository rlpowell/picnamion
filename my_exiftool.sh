#!/bin/bash

# We have this script because the -api stuff doesn't appear to work using
# -stay_open.  Might as well include the rest of our always-on options.
#
# The -d format string here causes all dates to be presented with time zones,
# even when the file does not, in fact, have any idea what the time zone for
# the given date is.  It uses the system default TZ in that case.  BUT.
#
# The -api part says "act like the system default TZ is
# https://en.wikipedia.org/wiki/UTC−12:00 ".  😄
#
# Which means that you can tell at a glance in the output whether a TZ was
# actually defined by something real in the metadata.  Like it's *technically*
# possible that someone took pictures in that time zone but not in a way that
# matters.
#
# The ##DATE## part is just because AFAICT there's no way to get exiftool to
# *say* "this is a date tag", even though it itself clearly knows; this way
# "gather all date tags" is easy.
exec exiftool -m -g0 -api TimeZone="GMT+12" -d '##DATE## %Y-%m-%d %H:%M:%S %z' "$@"

# Side comment: -time:all will show just the time-related entries
