#!/usr/bin/env bash
# Deploy the service to Cloud Run. Settings come from .env; see .env.example.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
set -a
source .env
set +a

gcloud run deploy "${TUBE_SERVICE:-tube}" --source . \
  --region "${TUBE_REGION}" --project "${TUBE_PROJECT}" \
  --allow-unauthenticated --port 8080 \
  --memory 256Mi --cpu 1 --min-instances 0 --max-instances 3 \
  --set-env-vars "TUBE_BUCKET=${TUBE_BUCKET}" \
  --set-secrets "/secrets/signer.json=${TUBE_SECRET}:latest"
