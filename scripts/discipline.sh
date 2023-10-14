#!/bin/zsh

set -e

CFG=${CFG:-/usr/local/etc/discipline.cfg}

fail() {
    echo "$0: $*" >&2
    exit 1
}

if [ -e $CFG ]; then
    . $CFG
else
    fail "Cannot find configuration file $CFG"
fi

t_alert_last=0

$CLIENT --sender-subject $KID \
	--show-time-remaining \
	--loop-delay $LOOP_DELAY \
	--retry-delay $RETRY_DELAY | while true ; do
    read T
    t_now=$(date +%s)

    
done
