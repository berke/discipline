# discipline

This is a system for enforcing computer usage time limits for children
on Linux desktop systems.

## Architecture

It consists of:

- A server that keeps track of the allowed time.  The server presents
a WebSocket interface with JSON command/response packets serialized
using Serde.  The server is necessary because computers can be turned
off, NAT traversal is annoying.
- An "administrator" client GUI (using GTK4 and tokio-tungstenite)
- A command-line client
- A shell script running in a systemd service that uses the above
command-line client.  It plays sounds to alert the user when the
remaining time crosses predefined thresholds, and kicks them out when
their time has expired.

## Operation

- You have to manually grant each kid a certain amount of computer
time through an interface (GUI or CLI.)
- The kid can then log in and use the computer.
- When the time gets low alerts are sounded.
- When the time expires the session is terminated.

An authorization for X seconds overrides any existing countdown
and sets the timer to X.  Thus an authorization for zero seconds
effectively cancels the computer time immediately.

## Security

This is a low-security system.  If your kids can figure out how to
spoof authorization packets they are probably old enough to control
their own screen time.

## Configuration

The server has a state file that gives the list of kids, their current
authorizations and the list of administrators.  This needs to be
initialized manually.

The UI has a configuration file that basically gives the WebSocket
URI of the server and the name of the "administrator" (i.e. mother
or father or guardian.)

None of this is well-documented for now, if there is interest I'll
clean it up.

## TODO

- Allow definition of a fixed authorized schedule
- Implement packet signatures (just for fun)
- Desktop icon and installer

Author: Berké DURAK <bd@exhrd.fr>
