#!/usr/bin/env bash
# Upload a video to the store and print its link.
#
#   ./upload.sh FILE.mp4 ["Title"]
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

file="$1"
# Eleven of base62. The store is private, so this is only what the page
# needs to ask for, not what keeps anyone out. cut rather than head, so
# nothing upstream dies of SIGPIPE under pipefail.
id="$(head -c 256 /dev/urandom | LC_ALL=C tr -dc 'A-Za-z0-9' | cut -c1-11)"

gcloud storage cp "${file}" "gs://${TUBE_BUCKET}/${id}.mp4" \
  --cache-control="public, max-age=31536000, immutable" \
  --content-type=video/mp4 --project="${TUBE_PROJECT}"

if [[ -n "${2:-}" ]]; then
  printf '%s' "$2" \
    | python3 -c 'import json,sys; print(json.dumps({"title": sys.stdin.read()}))' \
    | gcloud storage cp - "gs://${TUBE_BUCKET}/${id}.json" \
      --cache-control="public, max-age=300" \
      --content-type=application/json --project="${TUBE_PROJECT}"
fi

echo "https://${TUBE_HOST:-<host>}/${id}"
