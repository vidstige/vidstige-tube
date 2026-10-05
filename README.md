# tube

A very small place to put a video and send someone a link to it. One Node
service on Cloud Run serves a single page and signs URLs into a private
bucket; the browser streams the video straight from storage. Nothing is
listed anywhere, so a link is the only way in.

## What it needs

- A Google Cloud project with Cloud Run and Secret Manager enabled.
- A private bucket for the videos.
- A service account that can read the bucket, with one of its keys stored
  in Secret Manager. That key signs the links.

```bash
gcloud storage buckets create gs://BUCKET --location=REGION \
  --uniform-bucket-level-access
gcloud iam service-accounts create tube-signer
gcloud storage buckets add-iam-policy-binding gs://BUCKET \
  --member="serviceAccount:tube-signer@PROJECT.iam.gserviceaccount.com" \
  --role=roles/storage.objectViewer
gcloud iam service-accounts keys create key.json \
  --iam-account=tube-signer@PROJECT.iam.gserviceaccount.com
gcloud secrets create SECRET --data-file=key.json
rm key.json
```

## Running it

Copy `.env.example` to `.env` and fill it in. Then:

```bash
./deploy.sh                           # deploy the page
./deploy.sh add film.mp4 "Some film"  # upload a video, print its link
```

To run locally, point `TUBE_KEY` at a signer key file:

```bash
TUBE_BUCKET=BUCKET TUBE_KEY=key.json node server.js
```

## Videos

Encode for streaming: cap the bitrate and put the index at the front with
`-movflags +faststart`, or playback will wait for the whole file.

```bash
ffmpeg -i master.mp4 -c:v libx264 -preset slow -crf 20 \
  -maxrate 25M -bufsize 50M -pix_fmt yuv420p \
  -c:a aac -b:a 256k -movflags +faststart out.mp4
```
