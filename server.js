// Serves one page, and hands out signed links to a private video store.
//
// Nothing in the bucket is readable without a signature, so the only way to a
// video is through here: the page asks for an id, this signs a URL good for a
// few hours, and the browser then streams straight from storage. Proxying the
// bytes instead would put the whole file through this service for no benefit,
// since a signed URL honours range requests just the same.
//
// The signing key is a service account key mounted as a file — from Secret
// Manager in production — so nothing here ever talks to a secret store.

const crypto = require("crypto");
const fs = require("fs");
const http = require("http");
const path = require("path");

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is not set`);
  return value;
}

const PORT = process.env.PORT || 8080;
const BUCKET = required("TUBE_BUCKET");
const KEY_FILE = process.env.TUBE_KEY || "/secrets/signer.json";
const HOST = "storage.googleapis.com";
/** Long enough to watch twice over, short enough that a leaked link rots. */
const LIFETIME = Number(process.env.TUBE_LINK_SECONDS || 6 * 60 * 60);
const ID = /^[A-Za-z0-9]{6,32}$/;

const key = JSON.parse(fs.readFileSync(KEY_FILE, "utf8"));
const page = fs.readFileSync(path.join(__dirname, "index.html"));

/** RFC 3986, which is stricter about these six than encodeURIComponent is. */
const enc = (s) =>
  encodeURIComponent(s).replace(/[!'()*]/g,
    (c) => "%" + c.charCodeAt(0).toString(16).toUpperCase());

/** A V4 signed GET, as storage.googleapis.com wants it. */
function signed(object, seconds) {
  const stamp = new Date().toISOString().replace(/[-:]/g, "").replace(/\.\d+/, "");
  const scope = `${stamp.slice(0, 8)}/auto/storage/goog4_request`;
  const query = [
    ["X-Goog-Algorithm", "GOOG4-RSA-SHA256"],
    ["X-Goog-Credential", `${key.client_email}/${scope}`],
    ["X-Goog-Date", stamp],
    ["X-Goog-Expires", String(seconds)],
    ["X-Goog-SignedHeaders", "host"],
  ]
    .map(([k, v]) => [enc(k), enc(v)])
    .sort((a, b) => (a[0] < b[0] ? -1 : 1))
    .map(([k, v]) => `${k}=${v}`)
    .join("&");

  const resource = `/${BUCKET}/${object.split("/").map(enc).join("/")}`;
  const canonical =
    ["GET", resource, query, `host:${HOST}\n`, "host", "UNSIGNED-PAYLOAD"].join("\n");
  const toSign = [
    "GOOG4-RSA-SHA256",
    stamp,
    scope,
    crypto.createHash("sha256").update(canonical).digest("hex"),
  ].join("\n");
  const signature = crypto
    .createSign("RSA-SHA256")
    .update(toSign)
    .sign(key.private_key, "hex");

  return `https://${HOST}${resource}?${query}&X-Goog-Signature=${signature}`;
}

http
  .createServer((request, response) => {
    const url = new URL(request.url, `http://${request.headers.host}`);
    const link = url.pathname.match(/^\/s\/([^/]+)$/);

    if (link) {
      const id = link[1];
      if (!ID.test(id)) {
        response.writeHead(404).end();
        return;
      }
      response.writeHead(200, {
        "content-type": "application/json",
        "cache-control": "private, no-store",
      });
      response.end(JSON.stringify({
        video: signed(`${id}.mp4`, LIFETIME),
        meta: signed(`${id}.json`, LIFETIME),
      }));
      return;
    }

    response.writeHead(200, {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "public, max-age=300",
      "x-robots-tag": "noindex, nofollow",
    });
    response.end(page);
  })
  .listen(PORT);
