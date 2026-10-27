#!/usr/bin/env sh
# Prints a Kubernetes Secret on stdout; prompts and the generated webhook secret go to stderr.
set -eu
name=${SECRET_NAME:-pebble-index01-to-webdav}
ns=${NAMESPACE:-pebble}

ask() {
  printf '%s' "$1" >&2
  stty -echo 2>/dev/null || true
  read -r v || { echo 'no input: run this in a terminal' >&2; exit 1; }
  stty echo 2>/dev/null || true
  echo >&2
  printf '%s' "$v"
}

webhook=$(ask 'Pebble webhook secret (empty = generate): ')
if [ -z "$webhook" ]; then
  webhook=$(openssl rand -hex 32)
  echo "Paste this signing secret in the Pebble app: $webhook" >&2
fi
caldav=$(ask 'CalDAV app password: ')
[ -n "$caldav" ] || { echo 'CalDAV app password is required' >&2; exit 1; }

q() { printf "'%s'" "$(printf '%s' "$1" | sed "s/'/''/g")"; }  # YAML single-quoted

cat <<EOF
apiVersion: v1
kind: Secret
metadata:
  name: $name
  namespace: $ns
type: Opaque
stringData:
  PEBBLE_WEBHOOK_SECRET: $(q "$webhook")
  CALDAV_APP_PASSWORD: $(q "$caldav")
EOF
