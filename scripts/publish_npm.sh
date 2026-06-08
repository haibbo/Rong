#!/usr/bin/env bash
set -euo pipefail

PACKAGE_DIRS=("rong_types" "skill")

if [ -z "${NPM_TOKEN:-}" ] && [ -z "${NODE_AUTH_TOKEN:-}" ]; then
  echo "NPM_TOKEN or NODE_AUTH_TOKEN is required to publish npm packages" >&2
  exit 1
fi

export NODE_AUTH_TOKEN="${NODE_AUTH_TOKEN:-${NPM_TOKEN:-}}"

for package_dir in "${PACKAGE_DIRS[@]}"; do
  package_json="${package_dir}/package.json"

  package_name="$(node -p "require('./${package_json}').name")"
  package_version="$(node -p "require('./${package_json}').version")"

  if [ -z "$package_name" ] || [ -z "$package_version" ]; then
    echo "Unable to resolve npm package name/version from ${package_json}" >&2
    exit 1
  fi

  echo "Preparing npm publish for ${package_name}@${package_version}"

  existing_version="$(npm view "${package_name}@${package_version}" version 2>/dev/null || true)"
  if [ "$existing_version" = "$package_version" ]; then
    echo "Skipping ${package_name}@${package_version}; already published."
    continue
  fi

  (
    cd "$package_dir"
    npm install --no-package-lock
    npm publish --access public
  )
done
