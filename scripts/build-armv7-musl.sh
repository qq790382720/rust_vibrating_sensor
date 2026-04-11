#!/usr/bin/env bash
set -euo pipefail

TARGET="armv7-unknown-linux-musleabihf"
DEFAULT_IMAGE="rust-musl-cross:armv7-musleabihf"
FALLBACK_IMAGE="messense/rust-musl-cross:armv7-musleabihf"
IMAGE="${RUST_MUSL_CROSS_IMAGE:-}"
WORKDIR="/home/rust/src"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_PATH="$REPO_ROOT/target/$TARGET/release/rust_vibrating_sensor"

if ! command -v docker >/dev/null 2>&1; then
  echo "Error: docker command not found." >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "Error: Docker daemon is not running." >&2
  exit 1
fi

if [ -z "$IMAGE" ]; then
  for candidate in "$DEFAULT_IMAGE" "$FALLBACK_IMAGE"; do
    if docker image inspect "$candidate" >/dev/null 2>&1; then
      IMAGE="$candidate"
      break
    fi
  done
fi

if [ -z "$IMAGE" ]; then
  echo "Error: no supported ARMv7 musl builder image was found locally." >&2
  echo "Checked: $DEFAULT_IMAGE and $FALLBACK_IMAGE" >&2
  echo "Hint: set RUST_MUSL_CROSS_IMAGE to your local tag, or load/pull the image first." >&2
  exit 1
fi

BUILD_CMD=(
  cargo
  build
  --release
  --target
  "$TARGET"
)

if [ "$#" -gt 0 ]; then
  BUILD_CMD+=("$@")
fi

echo "Building rust_vibrating_sensor for $TARGET with image $IMAGE"

docker run --rm \
  -v "$REPO_ROOT:$WORKDIR" \
  -w "$WORKDIR" \
  "$IMAGE" \
  "${BUILD_CMD[@]}"

if [ ! -f "$ARTIFACT_PATH" ]; then
  echo "Error: build completed but artifact was not found at $ARTIFACT_PATH" >&2
  exit 1
fi

if command -v file >/dev/null 2>&1; then
  FILE_OUTPUT="$(file "$ARTIFACT_PATH")"
  echo "$FILE_OUTPUT"

  case "$FILE_OUTPUT" in
    *ARM*statically\ linked*)
      ;;
    *)
      echo "Error: artifact does not look like a statically linked ARM binary." >&2
      exit 1
      ;;
  esac
fi

echo "Build finished: $ARTIFACT_PATH"
