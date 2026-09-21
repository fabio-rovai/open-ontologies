#!/bin/sh
# Fetch the PINNED engine release for the function. The page runs the same
# binary a user would download; nothing here is a reimplementation of it.
set -eu
TAG="${OO_ENGINE_TAG:-v1.6.0}"
mkdir -p api/_bin
URL="https://github.com/fabio-rovai/open-ontologies/releases/download/${TAG}/open-ontologies-x86_64-unknown-linux-gnu"
echo "engine: ${URL}"
curl -fsSL -o api/_bin/open-ontologies "${URL}"
curl -fsSL -o api/_bin/SHASUMS.txt "https://github.com/fabio-rovai/open-ontologies/releases/download/${TAG}/SHASUMS.txt"

# Every file is checked against the release's own SHASUMS.txt before it is used.
# A demo that runs an unverified binary is a demo about trust.
verify() {
  want=$(grep "  $1\$" api/_bin/SHASUMS.txt | awk '{print $1}')
  got=$(sha256sum "api/_bin/$2" | awk '{print $1}')
  [ -n "$want" ] || { echo "no checksum for $1 in SHASUMS.txt"; exit 1; }
  [ "$want" = "$got" ] || { echo "checksum mismatch for $1: want $want got $got"; exit 1; }
  echo "  $2  $got"
}
verify "open-ontologies-x86_64-unknown-linux-gnu" "open-ontologies"
chmod +x api/_bin/open-ontologies

# The Lean certificate checker. The page runs it on the certificate the engine
# wrote, and on a forged copy of it, so the visitor sees both answers. Released
# from v1.6.0; before that tag the release carried the engine alone and the
# page says the checker is absent rather than pretending it ran.
CERT_ASSET="oo-cert-x86_64-unknown-linux-gnu"
if curl -fsSL -o api/_bin/oo-cert \
     "https://github.com/fabio-rovai/open-ontologies/releases/download/${TAG}/${CERT_ASSET}"; then
  verify "${CERT_ASSET}" "oo-cert"
  chmod +x api/_bin/oo-cert
  echo "checker ${TAG} pinned"
else
  rm -f api/_bin/oo-cert
  echo "::warning::${TAG} publishes no ${CERT_ASSET}; the page will report the checker as absent"
fi

echo "${TAG}" > api/_bin/TAG
echo "engine ${TAG} pinned"
