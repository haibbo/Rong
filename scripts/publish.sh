#!/usr/bin/env bash
set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
DRY_RUN=false
NO_VERIFY=false
ALLOW_DIRTY=false
WAIT_TIMEOUT=60  # Maximum wait time for crates.io sync
POLL_INTERVAL=5  # Check every 5 seconds

# Publishing order (topologically sorted by dependencies)
CRATES=(
  # Layer 1 - Foundation (no workspace deps)
  "rong_macro"
  "rong_core"

  # Layer 2 - System bindings
  "rong_quickjs_sys"
  "rong_jscore_sys"
  # "rong_arkjs_sys"  # TODO: Not ready yet

  # Layer 3 - Engine backends
  "rong_quickjs"
  "rong_jscore"
  # "rong_arkjs"  # TODO: Not ready yet

  # Layer 4 - Main runtime
  "rong"

  # Layer 5 - Basic modules
  "rong_console"
  "rong_assert"
  "rong_buffer"
  "rong_encoding"
  "rong_navigator"
  "rong_path"
  "rong_url"
  "rong_event"
  "rong_timer"

  # Layer 6 - Intermediate modules
  "rong_exception"
  "rong_abort"
  "rong_stream"

  # Layer 7 - Advanced modules
  "rong_fs"
  "rong_storage"
  "rong_process"
  "rong_child_process"
  "rong_http"

  # Layer 8 - Meta package
  "rong_modules"

  # NOTE: rong_test, rong_cli, and examples are NOT published
)

usage() {
  cat << EOF
Usage: $0 [OPTIONS]

Publish all workspace crates to crates.io in dependency order.

OPTIONS:
  -d, --dry-run         Run cargo publish with --dry-run (default: false)
  -n, --no-verify       Skip build verification with --no-verify (default: false)
  -a, --allow-dirty     Allow publishing with uncommitted changes (default: false)
  -t, --timeout SECONDS Maximum wait time for crates.io sync (default: 60)
  -p, --poll SECONDS    Poll interval for checking crates.io (default: 5)
  -h, --help            Show this help message

EXAMPLES:
  # Dry run to test without publishing
  $0 --dry-run

  # Publish with smart waiting (recommended)
  $0 --allow-dirty

  # Fast publish without verification
  $0 --no-verify --allow-dirty

  # Custom timeout settings
  $0 --timeout 120 --poll 3 --allow-dirty
EOF
  exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    -d|--dry-run)
      DRY_RUN=true
      shift
      ;;
    -n|--no-verify)
      NO_VERIFY=true
      shift
      ;;
    -a|--allow-dirty)
      ALLOW_DIRTY=true
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

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  RongJS Workspace Publishing Script${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "Total crates to publish: ${GREEN}${#CRATES[@]}${NC}"
echo -e "Dry run: ${YELLOW}${DRY_RUN}${NC}"
echo -e "No verify: ${YELLOW}${NO_VERIFY}${NC}"
echo -e "Allow dirty: ${YELLOW}${ALLOW_DIRTY}${NC}"
echo -e "Sync timeout: ${YELLOW}${WAIT_TIMEOUT}s${NC}"
echo -e "Poll interval: ${YELLOW}${POLL_INTERVAL}s${NC}"
echo ""

# Check if CARGO_REGISTRY_TOKEN is set
if [ -z "${CARGO_REGISTRY_TOKEN:-}" ] && [ "$DRY_RUN" = false ]; then
  echo -e "${RED}ERROR: CARGO_REGISTRY_TOKEN is not set${NC}"
  echo "Please set it with: export CARGO_REGISTRY_TOKEN=your_token"
  echo "Get your token from: https://crates.io/me"
  exit 1
fi

# Confirmation prompt
if [ "$DRY_RUN" = false ]; then
  echo -e "${YELLOW}⚠️  This will publish ${#CRATES[@]} crates to crates.io!${NC}"
  read -p "Are you sure you want to continue? (yes/no): " -r
  echo
  if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    echo "Aborted."
    exit 1
  fi
fi

# Function to wait for crate to appear in crates.io index
wait_for_crate() {
  local crate=$1
  local timeout=$2
  local poll_interval=$3
  local elapsed=0

  echo -e "${YELLOW}Waiting for ${crate} to appear in crates.io index...${NC}"

  while [ $elapsed -lt "$timeout" ]; do
    # Try to search for the crate
    if cargo search "$crate" --limit 1 2>/dev/null | grep -q "^$crate = "; then
      echo -e "${GREEN}✓ ${crate} found in index after ${elapsed}s${NC}"
      return 0
    fi

    sleep "$poll_interval"
    elapsed=$((elapsed + poll_interval))
    echo -e "${YELLOW}  Still waiting... (${elapsed}s/${timeout}s)${NC}"
  done

  echo -e "${RED}⚠ Timeout waiting for ${crate} (waited ${timeout}s)${NC}"
  echo -e "${YELLOW}  Continuing anyway, verification might fail...${NC}"
  return 1
}

# Publish each crate
PUBLISHED=0
FAILED=0

for crate in "${CRATES[@]}"; do
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${BLUE}Publishing [$((PUBLISHED + 1))/${#CRATES[@]}]: ${GREEN}${crate}${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

  # Build cargo publish command
  cmd=(cargo publish -p "$crate")

  if [ "$DRY_RUN" = true ]; then
    cmd+=(--dry-run)
  fi

  if [ "$NO_VERIFY" = true ]; then
    cmd+=(--no-verify)
  fi

  if [ "$ALLOW_DIRTY" = true ]; then
    cmd+=(--allow-dirty)
  fi

  echo -e "${YELLOW}Running: ${cmd[*]}${NC}"

  # Execute publish command
  if "${cmd[@]}"; then
    PUBLISHED=$((PUBLISHED + 1))
    echo -e "${GREEN}✓ Successfully published ${crate}${NC}"

    # Wait for crates.io to sync (except for last crate and dry-run)
    if [ $PUBLISHED -lt ${#CRATES[@]} ] && [ "$DRY_RUN" = false ]; then
      wait_for_crate "$crate" "$WAIT_TIMEOUT" "$POLL_INTERVAL"
    fi
  else
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
echo -e "${GREEN}✓ Successfully published: ${PUBLISHED}/${#CRATES[@]}${NC}"
if [ $FAILED -gt 0 ]; then
  echo -e "${RED}✗ Failed: ${FAILED}${NC}"
  exit 1
else
  echo -e "${GREEN}🎉 All crates published successfully!${NC}"
fi
