#!/usr/bin/env bash
set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
NO_VERIFY=false
ALLOW_DIRTY=false
AUTO_CONFIRM=false
WAIT_TIMEOUT=180  # Maximum wait time for crates.io sync
POLL_INTERVAL=5  # Check every 5 seconds

WORKSPACE_TOML="Cargo.toml"

# Publishing order (topologically sorted by dependencies)
CRATES=(
  # Layer 1 - Foundation (no workspace deps)
  "rong_macro"
  "rong_rt"
  "rong_core"

  # Layer 2 - System bindings
  "rong_quickjs_sys"
  "rong_jscore_sys"
  "rong_arkjs_sys"

  # Layer 3 - Engine backends
  "rong_quickjs"
  "rong_jscore"
  "rong_arkjs"

  # Layer 4 - Main runtime facade
  "rong"

  # Layer 5 - Basic modules
  "rong_console"
  "rong_assert"
  "rong_encoding"
  "rong_url"
  "rong_timer"
  "rong_event"
  "rong_buffer"

  # Layer 6 - Intermediate modules
  "rong_exception"
  "rong_abort"
  "rong_stream"

  # Layer 7 - Advanced modules
  "rong_fs"
  "rong_storage"
  "rong_http"
  "rong_compression"
  "rong_command"
  "rong_redis"
  "rong_sqlite"
  "rong_worker"
  "rong_s3"
  # Layer 8 - Meta package
  "rong_modules"
  "rong_cli"

  # NOTE: rong_test and examples are NOT published
)

usage() {
  cat << EOF
Usage: $0 [OPTIONS]

Publish all workspace crates to crates.io in dependency order.

OPTIONS:
  -n, --no-verify       Skip build verification with --no-verify (default: false)
  -a, --allow-dirty     Allow publishing with uncommitted changes (default: false)
  -y, --yes             Skip confirmation prompt (default: false)
  -t, --timeout SECONDS Maximum wait time for crates.io sync (default: 180)
  -p, --poll SECONDS    Poll interval for checking crates.io (default: 5)
  -h, --help            Show this help message

EXAMPLES:
  # Publish with verification (recommended)
  $0

  # Fast publish without verification
  $0 --no-verify

  # Publish with uncommitted changes
  $0 --allow-dirty

  # Non-interactive publish
  $0 --yes

  # Custom timeout settings
  $0 --timeout 120 --poll 3
EOF
  exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    -n|--no-verify)
      NO_VERIFY=true
      shift
      ;;
    -a|--allow-dirty)
      ALLOW_DIRTY=true
      shift
      ;;
    -y|--yes)
      AUTO_CONFIRM=true
      shift
      ;;
    -t|--timeout)
      WAIT_TIMEOUT="$2"
      shift 2
      ;;
    -p|--poll)
      POLL_INTERVAL="$2"
      shift 2
      ;;
    -h|--help)
      usage
      ;;
    *)
      echo "Unknown option: $1"
      usage
      ;;
  esac
done

# Confirmation prompt (auto-approve in CI)
if [ "${CI:-}" = "true" ]; then
  AUTO_CONFIRM=true
fi

# Extract the expected version so we can wait for crates.io index to reflect the
# publish we just did (not merely the crate's existence).
EXPECTED_VERSION="$(grep -A 2 '^\[workspace.package\]' "$WORKSPACE_TOML" 2>/dev/null | grep '^version' | sed 's/version = "\(.*\)"/\1/' || true)"
if [ -z "$EXPECTED_VERSION" ]; then
  echo -e "${YELLOW}⚠️  Could not read workspace.package.version from ${WORKSPACE_TOML}; crates.io sync waiting may be unreliable.${NC}"
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  RongJS Workspace Publishing Script${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "Total crates to publish: ${GREEN}${#CRATES[@]}${NC}"
echo -e "No verify: ${YELLOW}${NO_VERIFY}${NC}"
echo -e "Allow dirty: ${YELLOW}${ALLOW_DIRTY}${NC}"
echo -e "Auto confirm: ${YELLOW}${AUTO_CONFIRM}${NC}"
echo -e "Expected version: ${YELLOW}${EXPECTED_VERSION:-unknown}${NC}"
echo -e "Sync timeout: ${YELLOW}${WAIT_TIMEOUT}s${NC}"
echo -e "Poll interval: ${YELLOW}${POLL_INTERVAL}s${NC}"
echo ""

# Check if CARGO_REGISTRY_TOKEN is set
if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
  echo -e "${RED}ERROR: CARGO_REGISTRY_TOKEN is not set${NC}"
  echo "Please set it with: export CARGO_REGISTRY_TOKEN=your_token"
  echo "Get your token from: https://crates.io/me"
  exit 1
fi

if [ "$AUTO_CONFIRM" = true ]; then
  echo -e "${YELLOW}⚠️  Auto-confirm enabled; proceeding with publish.${NC}"
else
  echo -e "${YELLOW}⚠️  This will publish ${#CRATES[@]} crates to crates.io!${NC}"
  read -p "Are you sure you want to continue? (yes/no): " -r
  echo
  if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    echo "Aborted."
    exit 1
  fi
fi

# Function to check if a crate version is already published
is_crate_published() {
  local crate=$1
  local version=$2

  if [ -z "$version" ]; then
    # No version specified, just check if crate exists
    if cargo search "$crate" --limit 1 2>/dev/null | grep -q "^$crate = "; then
      return 0
    fi
  else
    # Check for specific version
    if cargo search "$crate" --limit 1 2>/dev/null | grep -q "^$crate = \"${version}\""; then
      return 0
    fi
  fi
  return 1
}

# Function to wait for crate to appear in crates.io index
wait_for_crate() {
  local crate=$1
  local version=$2
  local timeout=$3
  local poll_interval=$4
  local elapsed=0

  if [ -z "$version" ]; then
    echo -e "${YELLOW}Waiting for ${crate} to appear in crates.io index...${NC}"
  else
    echo -e "${YELLOW}Waiting for ${crate} ${version} to appear in crates.io index...${NC}"
  fi

  while [ $elapsed -lt "$timeout" ]; do
    # Try to search for the crate
    if [ -z "$version" ]; then
      if cargo search "$crate" --limit 1 2>/dev/null | grep -q "^$crate = "; then
        echo -e "${GREEN}✓ ${crate} found in index after ${elapsed}s${NC}"
        return 0
      fi
    else
      if cargo search "$crate" --limit 1 2>/dev/null | grep -q "^$crate = \"${version}\""; then
        echo -e "${GREEN}✓ ${crate} ${version} found in index after ${elapsed}s${NC}"
        return 0
      fi
    fi

    sleep "$poll_interval"
    elapsed=$((elapsed + poll_interval))
    echo -e "${YELLOW}  Still waiting... (${elapsed}s/${timeout}s)${NC}"
  done

  if [ -z "$version" ]; then
    echo -e "${RED}⚠ Timeout waiting for ${crate} (waited ${timeout}s)${NC}"
  else
    echo -e "${RED}⚠ Timeout waiting for ${crate} ${version} (waited ${timeout}s)${NC}"
  fi
  echo -e "${YELLOW}  Continuing anyway, verification might fail...${NC}"
  return 1
}

# Publish each crate
PUBLISHED=0
SKIPPED=0
FAILED=0

for crate in "${CRATES[@]}"; do
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${BLUE}Publishing [$((PUBLISHED + SKIPPED + 1))/${#CRATES[@]}]: ${GREEN}${crate}${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

  # Check if crate is already published and skip it
  if is_crate_published "$crate" "$EXPECTED_VERSION"; then
    echo -e "${YELLOW}⊘ Skipping ${crate} ${EXPECTED_VERSION} (already published)${NC}"
    SKIPPED=$((SKIPPED + 1))
    continue
  fi

  # Build cargo publish command
  cmd=(cargo publish -p "$crate")

  if [ "$NO_VERIFY" = true ]; then
    cmd+=(--no-verify)
  fi

  if [ "$ALLOW_DIRTY" = true ]; then
    cmd+=(--allow-dirty)
  fi

  echo -e "${YELLOW}Running: ${cmd[*]}${NC}"

  # Execute publish command
  if publish_output="$("${cmd[@]}" 2>&1)"; then
    printf '%s\n' "$publish_output"
    PUBLISHED=$((PUBLISHED + 1))
    echo -e "${GREEN}✓ Successfully published ${crate}${NC}"

    # Wait for crates.io to sync (except for last crate)
    if [ $((PUBLISHED + SKIPPED)) -lt ${#CRATES[@]} ]; then
      if ! wait_for_crate "$crate" "$EXPECTED_VERSION" "$WAIT_TIMEOUT" "$POLL_INTERVAL"; then
        echo -e "${YELLOW}⚠ Proceeding despite crates.io index lag for ${crate}.${NC}"
      fi
    fi
  else
    printf '%s\n' "$publish_output"
    if grep -Eq 'already exists on crates\.io index|already uploaded' <<<"$publish_output"; then
      SKIPPED=$((SKIPPED + 1))
      echo -e "${YELLOW}⊘ Skipping ${crate} ${EXPECTED_VERSION} (already published; detected during cargo publish)${NC}"
      continue
    fi

    FAILED=$((FAILED + 1))
    echo -e "${RED}✗ Failed to publish ${crate}${NC}"
    echo -e "${RED}Stopping publish process due to error.${NC}"
    exit 1
  fi
done

# Summary
echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Publishing Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}✓ Successfully published: ${PUBLISHED}${NC}"
if [ $SKIPPED -gt 0 ]; then
  echo -e "${YELLOW}⊘ Skipped (already published): ${SKIPPED}${NC}"
fi
if [ $FAILED -gt 0 ]; then
  echo -e "${RED}✗ Failed: ${FAILED}${NC}"
  exit 1
else
  if [ $SKIPPED -gt 0 ]; then
    echo -e "${GREEN}🎉 All crates processed! (${PUBLISHED} published, ${SKIPPED} skipped)${NC}"
  else
    echo -e "${GREEN}🎉 All crates published successfully!${NC}"
  fi
fi
