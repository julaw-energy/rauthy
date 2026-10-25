#!/usr/bin/env bash
# Builds our image locally from source, exactly as the GitHub workflow does.
#
#   tools/image/build.sh                      -> rauthy-custom:local (loaded into the local docker)
#   tools/image/build.sh --tag ghcr.io/me/rauthy:test
#   tools/image/build.sh --push --tag ghcr.io/me/rauthy:1   (needs `docker login ghcr.io` first)
#
# The first build takes a while (Rust release build, UI, docs). Later ones reuse the layers.
set -euo pipefail
cd "$(dirname "$0")/../.."

TAG="rauthy-custom:local"
ACTION="--load"
EXTRA=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --tag) TAG="$2"; shift 2 ;;
        --push) ACTION="--push"; shift ;;
        --platform) EXTRA+=(--platform "$2"); shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

VERSION="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)-custom.$(git rev-parse --short HEAD)"
if ! git diff --quiet HEAD -- . ':!tools'; then
    echo "NOTE: there are uncommitted changes, the image contains them (version $VERSION-dirty)"
    VERSION="$VERSION-dirty"
fi

# The daemon's own builder by default. Set BUILDER=<name> for another one, for example a
# docker-container builder when you build for more than one platform.
DOCKER_BUILDKIT=1 docker buildx build \
    --builder "${BUILDER:-default}" \
    -f Dockerfile.fork \
    --build-arg CREATED="$(date --rfc-3339=seconds)" \
    --build-arg VERSION="$VERSION" \
    -t "$TAG" \
    "${EXTRA[@]}" \
    "$ACTION" \
    .

echo "Built $TAG ($VERSION)"
