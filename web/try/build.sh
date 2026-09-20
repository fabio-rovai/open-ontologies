#!/bin/sh
# Fetch the PINNED engine release for the function. The page runs the same
# binary a user would download; nothing here is a reimplementation of it.
set -eu
TAG="${OO_ENGINE_TAG:-v1.5.0}"
mkdir -p api/_bin
URL="https://github.com/fabio-rovai/open-ontologies/releases/download/${TAG}/open-ontologies-x86_64-unknown-linux-gnu"
echo "engine: ${URL}"
curl -fsSL -o api/_bin/open-ontologies "${URL}"
curl -fsSL -o api/_bin/SHASUMS.txt "https://github.com/fabio-rovai/open-ontologies/releases/download/${TAG}/SHASUMS.txt"
WANT=$(grep 'x86_64-unknown-linux-gnu' api/_bin/SHASUMS.txt | awk '{print $1}')
GOT=$(sha256sum api/_bin/open-ontologies | awk '{print $1}')
[ "$WANT" = "$GOT" ] || { echo "checksum mismatch: want $WANT got $GOT"; exit 1; }
chmod +x api/_bin/open-ontologies
echo "${TAG}" > api/_bin/TAG
echo "engine ${TAG} pinned, sha256 ${GOT}"
