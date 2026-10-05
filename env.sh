# Sourced by deploy.sh and upload.sh: loads .env and checks it is complete.
#
#   TUBE_PROJECT=...   Google Cloud project
#   TUBE_REGION=...    Cloud Run region
#   TUBE_BUCKET=...    private bucket holding the videos
#   TUBE_SECRET=...    Secret Manager secret holding the signer key
#   TUBE_SERVICE=...   Cloud Run service name
#   TUBE_HOST=...      domain the service answers on, for printed links
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
set -a
source "${root}/.env"
set +a

for required in TUBE_PROJECT TUBE_REGION TUBE_BUCKET TUBE_SECRET; do
  [[ -n "${!required:-}" ]] || {
    echo "${required} is not set; see .env.example" >&2
    exit 1
  }
done
