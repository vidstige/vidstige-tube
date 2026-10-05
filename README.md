# tube

A very small place to put a video and send someone a link to it.

One page on Cloud Run, one private bucket. The page takes an id out of its own
path, asks the service for a signed URL, and the browser streams straight from
storage. Nothing is listed anywhere and nothing in the bucket is readable
without a signature, so a link is the only way in — and it is only a link, not
a password: anyone it is forwarded to can watch.

Why signed URLs rather than serving the bytes: a video is large and a signed
URL honours range requests exactly as a public object would, so seeking works
and none of the traffic goes through the service.

## What it needs

- A Google Cloud project with Cloud Run and Secret Manager enabled.
- A **private** bucket for the videos — no `allUsers` binding at all.
- A service account that can read that bucket, and one of its keys stored in
  Secret Manager. That key is what signs the links.
- Optionally a domain mapped to the service, which gets a managed certificate.

Granting `allUsers` the obvious `roles/storage.objectViewer` would be a mistake:
it includes `storage.objects.list`, so anyone could read off every id in the
store. This design gives the public nothing and the service account
`roles/storage.objectViewer` on the bucket alone.

## Setting it up

```bash
gcloud storage buckets create gs://BUCKET --location=REGION --uniform-bucket-level-access
gcloud iam service-accounts create tube-signer
gcloud storage buckets add-iam-policy-binding gs://BUCKET \
  --member="serviceAccount:tube-signer@PROJECT.iam.gserviceaccount.com" \
  --role=roles/storage.objectViewer

gcloud iam service-accounts keys create key.json \
  --iam-account=tube-signer@PROJECT.iam.gserviceaccount.com
gcloud secrets create SECRET --data-file=key.json --replication-policy=automatic
shred -u key.json

gcloud secrets add-iam-policy-binding SECRET \
  --member="serviceAccount:RUNTIME-SERVICE-ACCOUNT" \
  --role=roles/secretmanager.secretAccessor
```

Then copy `.env.example` to `.env`, fill it in, and:

```bash
./deploy.sh                           # deploy the page
./deploy.sh add film.mp4 "Some film"  # upload a video, print its link
```

## Videos

An id is eleven characters of base62, and the store holds `<id>.mp4` beside an
optional `<id>.json` of `{"title": ..., "meta": ...}` for what the page shows
underneath.

Encode for streaming rather than for archiving — cap the peak bitrate and put
the index at the front, or the first seek will stall:

```bash
ffmpeg -i master.mp4 -c:v libx264 -preset slow -crf 20 \
  -maxrate 25M -bufsize 50M -profile:v high -level 4.2 -pix_fmt yuv420p \
  -g 60 -keyint_min 60 -c:a aac -b:a 256k -movflags +faststart out.mp4
```

`-movflags +faststart` is the one that matters: it moves the index ahead of the
media so playback can start before the file has arrived.

## A note on the key

A long-lived service account key is the part Google would rather you did not
have. The same signing works with no key at all if the runtime service account
is granted `roles/iam.serviceAccountTokenCreator` on itself and signs through
the IAM `signBlob` API, which would let you delete both the key and the secret.
