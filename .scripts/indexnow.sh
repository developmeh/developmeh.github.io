#!/usr/bin/env bash
#
# Submit the live sitemap's URLs to IndexNow.
#
# IndexNow is a push protocol supported by Bing and Yandex. Google does not
# participate, and both Google and Bing retired their sitemap ping endpoints,
# so robots.txt discovery remains the mechanism everywhere else. The value here
# is Bing-shaped, which is Copilot-shaped.
#
# Reads the LIVE sitemap rather than a local build, so it can only ever submit
# URLs that are actually published. Run it after deployment, never before.
#
#   INDEXNOW_KEY=<key> .scripts/indexnow.sh          # submit
#   INDEXNOW_KEY=<key> .scripts/indexnow.sh -n       # dry run, prints payload
#
# Env:
#   INDEXNOW_KEY       required, the key whose value is served at
#                      https://<host>/<key>.txt
#   INDEXNOW_HOST      default developmeh.com
#   INDEXNOW_SITEMAP   default https://<host>/sitemap.xml
#   INDEXNOW_ENDPOINT  default https://api.indexnow.org/indexnow
#
# Exits non-zero on a real failure. A non-2xx from the endpoint is reported but
# does NOT fail the run: search-engine submission is best-effort and must never
# break a deploy that has already succeeded.

set -euo pipefail

DRY_RUN=0
[ "${1:-}" = "-n" ] && DRY_RUN=1

HOST="${INDEXNOW_HOST:-developmeh.com}"
SITEMAP="${INDEXNOW_SITEMAP:-https://${HOST}/sitemap.xml}"
ENDPOINT="${INDEXNOW_ENDPOINT:-https://api.indexnow.org/indexnow}"

if [ -z "${INDEXNOW_KEY:-}" ]; then
  echo "INDEXNOW_KEY is not set" >&2
  exit 1
fi

KEY_LOCATION="https://${HOST}/${INDEXNOW_KEY}.txt"

echo "sitemap:  ${SITEMAP}"
echo "key file: ${KEY_LOCATION}"

# The key file must be reachable, or every submission is rejected.
if [ "$DRY_RUN" -eq 0 ]; then
  key_status="$(curl -fsS -o /dev/null -w '%{http_code}' "$KEY_LOCATION" || echo 000)"
  if [ "$key_status" != "200" ]; then
    echo "key file not reachable (HTTP ${key_status}); nothing submitted" >&2
    exit 0
  fi
fi

# `|| true` on the grep: no matches exits 1, which under `set -e` would kill
# the script before the friendly "no URLs" message below can fire.
sitemap_xml="$(curl -fsS "$SITEMAP")"
urls="$(printf '%s' "$sitemap_xml" | grep -o '<loc>[^<]*</loc>' | sed 's|</\?loc>||g' || true)"

count="$(printf '%s\n' "$urls" | grep -c . || true)"
if [ "$count" -eq 0 ]; then
  echo "no URLs found in sitemap; nothing to submit" >&2
  exit 1
fi
echo "urls:     ${count}"

payload="$(printf '%s\n' "$urls" | python3 -c '
import json, sys
urls = [line.strip() for line in sys.stdin if line.strip()]
print(json.dumps({
    "host": sys.argv[1],
    "key": sys.argv[2],
    "keyLocation": sys.argv[3],
    "urlList": urls,
}))' "$HOST" "$INDEXNOW_KEY" "$KEY_LOCATION")"

if [ "$DRY_RUN" -eq 1 ]; then
  echo "--- dry run, payload ---"
  printf '%s\n' "$payload" | python3 -m json.tool
  exit 0
fi

status="$(curl -sS -X POST "$ENDPOINT" \
  -H 'Content-Type: application/json; charset=utf-8' \
  -d "$payload" \
  -o /tmp/indexnow-response -w '%{http_code}' || echo 000)"

echo "response: HTTP ${status}"
[ -s /tmp/indexnow-response ] && cat /tmp/indexnow-response

case "$status" in
  200|202) echo "submitted ${count} urls" ;;
  400) echo "bad request: check the key format" >&2 ;;
  403) echo "key rejected: ${KEY_LOCATION} must contain exactly the key" >&2 ;;
  422) echo "url/host mismatch: urls must be on ${HOST}" >&2 ;;
  429) echo "rate limited; try again later" >&2 ;;
  *)   echo "unexpected status, treating as non-fatal" >&2 ;;
esac

# Never fail a deploy over a best-effort submission.
exit 0
