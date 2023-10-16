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

is_logged_in() {
    loginctl \
	list-sessions -o json |
	jq '.[] | select(.user == "'"$KID_USER"'").session' |
	tr -d '"' |
	grep -qv '^"[0-9]\\+"$'
}

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

sound_alert() {
    if [ $DRY_RUN = 1 ]; then
	msg "Would be sounding alert $1"
    else
	msg "Alert $1"
	su $KID_USER -c "XDG_RUNTIME_DIR=/run/user/1000 aplay $1 >/dev/null 2>&1" || true &
    fi
}

alert() {
    alert_file=$ALERT_PATH/${ALERT_FILES[$1]}
    alert_file=${alert_file:a}
    sound_alert $alert_file
}

# Find the largest applicable alert index given the remaining time
#
# Alert thresholds are real numbers t_1 > t_2 > ... > t_m
#
# Find the largest i such that
#  t_i > t > t_{i+1}

find_alert_index() {
    local i
    local thr
    local T=$1
    alert_index=0
    for (( i=1; i<=$#ALERT_THRESHOLDS; i++ )) ; do
	(( thr=${ALERT_THRESHOLDS[i]} ))
	if (( T <= thr )) ; then
	    alert_index=$i
	fi
    done
    msg "Alert index for $1 is $alert_index"
}

last_alert_index=0

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
	# Ran out of time
	# Kick user
	kick
	last_alert_index=0
	continue
    fi

    if is_logged_in ; then
	find_alert_index $T

	msg "$alert_index $last_alert_index al"
	if (( alert_index != last_alert_index )) ; then
	    if (( alert_index > last_alert_index )) ; then
		alert $alert_index
	    fi
	    last_alert_index=$alert_index
	fi
    fi
done
