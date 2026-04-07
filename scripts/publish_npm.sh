#!/usr/bin/env bash
set -euo pipefail

PACKAGE_DIR="rong_types"
PACKAGE_JSON="${PACKAGE_DIR}/package.json"

PACKAGE_NAME="$(sed -n 's/  "name": "\(.*\)",/\1/p' "$PACKAGE_JSON" | head -n 1)"
PACKAGE_VERSION="$(sed -n 's/  "version": "\(.*\)",/\1/p' "$PACKAGE_JSON" | head -n 1)"

if [ -z "$PACKAGE_NAME" ] || [ -z "$PACKAGE_VERSION" ]; then
  echo "Unable to resolve npm package name/version from ${PACKAGE_JSON}" >&2
  exit 1
fi

if [ -z "${NPM_TOKEN:-}" ] && [ -z "${NODE_AUTH_TOKEN:-}" ]; then
  echo "NPM_TOKEN or NODE_AUTH_TOKEN is required to publish ${PACKAGE_NAME}" >&2
  exit 1
fi

export NODE_AUTH_TOKEN="${NODE_AUTH_TOKEN:-${NPM_TOKEN:-}}"

echo "Preparing npm publish for ${PACKAGE_NAME}@${PACKAGE_VERSION}"

existing_version="$(npm view "${PACKAGE_NAME}@${PACKAGE_VERSION}" version 2>/dev/null || true)"
if [ "$existing_version" = "$PACKAGE_VERSION" ]; then
  echo "Skipping ${PACKAGE_NAME}@${PACKAGE_VERSION}; already published."
  exit 0
fi

cd "$PACKAGE_DIR"
npm install --no-package-lock
npm publish --access public
