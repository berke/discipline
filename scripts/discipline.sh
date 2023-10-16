#!/bin/zsh

set -e

CFG=${CFG:-/usr/local/etc/discipline/discipline.cfg}

fail() {
    echo "$0: $*" >&2
    exit 1
}

msg() {
    echo "-- $*"
}

if [ -e $CFG ]; then
    . $CFG
else
    fail "Cannot find configuration file $CFG"
fi

t_alert_last=0

kick() {
    if [ $DRY_RUN = 1 ]; then
	msg "Would be kicking $KID_USER"
    else
	loginctl \
	    list-sessions -o json |
	    jq '.[] | select(.user == "'"$KID_USER"'").session' |
	    tr -d '"' |
	    while read sess ; do
		msg "Kicking $KID_USER from session '$sess'"
		loginctl terminate-session $sess || true
	    done
    fi
}

alert() {
    if [ $DRY_RUN = 1 ]; then
	msg "Would be sounding alert"
    else
	su $KID_USER -c "XDG_RUNTIME_DIR=/run/user/1000 aplay ${ALERT:a}" || true
    fi
}

ialert=1

$CLIENT --url $URL \
	--sender-subject $KID \
	--subject $KID \
	--show-time-remaining \
	--loop-delay $LOOP_DELAY \
	--retry-delay $RETRY_DELAY | while read T ; do
    if [ -z "$T" ]; then
	msg "Empty response"
	continue
    fi
    
    if [ $T = 0 ]; then
	kick
	continue
    fi

    do_alert=0
    for (( i=ialert; i<=$#ALERT_THRESHOLDS; i++ )) ; do
	(( thr=${ALERT_THRESHOLDS[ialert]} ))
	if (( T <= thr )) ; then
	    do_alert=1
	    (( ialert=i+1 ))
	fi
    done
    if (( do_alert != 0 )) ; then
	alert
    fi
done
