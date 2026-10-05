#!/usr/bin/env bash
# Deploy the service to Cloud Run. Videos are a separate matter: upload.sh.
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

gcloud run deploy "${TUBE_SERVICE:-tube}" --source "${root}" \
  --region "${TUBE_REGION}" --project "${TUBE_PROJECT}" \
  --allow-unauthenticated --port 8080 \
  --memory 256Mi --cpu 1 --min-instances 0 --max-instances 3 \
  --set-env-vars "TUBE_BUCKET=${TUBE_BUCKET}" \
  --set-secrets "/secrets/signer.json=${TUBE_SECRET}:latest"
