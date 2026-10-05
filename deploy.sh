#!/usr/bin/env bash
# Deploy the page to Cloud Run, or add a video to the store.
#
# Settings live in .env, which is not committed:
#
#   TUBE_PROJECT=...        # Google Cloud project
#   TUBE_REGION=...         # Cloud Run region
#   TUBE_BUCKET=...         # private bucket holding the videos
#   TUBE_SECRET=...         # Secret Manager secret holding the signer key
#   TUBE_HOST=...           # domain the service answers on, for printed links
#
# Usage:
#   ./deploy.sh                       redeploy the page
#   ./deploy.sh add FILE.mp4 "Title"  upload a video, print its link
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "${root}/.env" ]]; then
  set -a
  source "${root}/.env"
  set +a
fi

for required in TUBE_PROJECT TUBE_REGION TUBE_BUCKET TUBE_SECRET; do
  [[ -n "${!required:-}" ]] || { echo "${required} is not set; see .env.example" >&2; exit 1; }
done

if [[ "${1:-}" == "add" ]]; then
  file="$2"
  # Eleven of base62. The store is private, so this is only what the page needs
  # to ask for, not what keeps anyone out.
  id="$(LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c 11)"
  gcloud storage cp "${file}" "gs://${TUBE_BUCKET}/${id}.mp4" \
    --cache-control="public, max-age=31536000, immutable" \
    --content-type=video/mp4 --project="${TUBE_PROJECT}"
  if [[ -n "${3:-}" ]]; then
    printf '%s' "$3" \
      | python3 -c 'import json,sys; print(json.dumps({"title": sys.stdin.read()}))' \
      | gcloud storage cp - "gs://${TUBE_BUCKET}/${id}.json" \
        --cache-control="public, max-age=300" \
        --content-type=application/json --project="${TUBE_PROJECT}"
  fi
  echo "https://${TUBE_HOST:-<host>}/${id}"
  exit 0
fi

gcloud run deploy "${TUBE_SERVICE:-tube}" --source "${root}" \
  --region "${TUBE_REGION}" --project "${TUBE_PROJECT}" \
  --allow-unauthenticated --port 8080 \
  --memory 256Mi --cpu 1 --min-instances 0 --max-instances 3 \
  --set-env-vars "TUBE_BUCKET=${TUBE_BUCKET}" \
  --set-secrets "/secrets/signer.json=${TUBE_SECRET}:latest"
