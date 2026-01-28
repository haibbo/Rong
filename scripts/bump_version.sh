#!/usr/bin/env bash
set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

WORKSPACE_TOML="Cargo.toml"

usage() {
  cat << EOF
Usage: $0 <new-version> [OPTIONS]

Bump workspace version and prepare for publishing.

ARGUMENTS:
  <new-version>         New version number (e.g., 0.1.2, 0.2.0, 1.0.0)

OPTIONS:
  --commit              Create git commit (disabled by default)
  --tag                 Create git tag (disabled by default)
  --commit-and-tag      Create both commit and tag (shortcut)
  -h, --help            Show this help message

EXAMPLES:
  # Dry run: just update Cargo.toml (default)
  $0 0.1.2

  # Update and commit
  $0 0.1.2 --commit

  # Update, commit, and tag
  $0 0.1.2 --commit-and-tag

  # Update and tag (without commit)
  $0 0.1.2 --tag

WORKFLOW:
  1. Updates [workspace.package] version
  2. Syncs all [workspace.dependencies] versions
  3. Creates git commit (if --commit or --commit-and-tag)
  4. Creates git tag v<version> (if --tag or --commit-and-tag)

DEFAULT BEHAVIOR:
  By default, this script only updates Cargo.toml without git operations.
  You must explicitly use --commit and/or --tag for git operations.
EOF
  exit 0
}

# Parse arguments
NEW_VERSION=""
DO_COMMIT=false
DO_TAG=false

while [[ $# -gt 0 ]]; do
  case $1 in
    -h|--help)
      usage
      ;;
    --commit)
      DO_COMMIT=true
      shift
      ;;
    --tag)
      DO_TAG=true
      shift
      ;;
    --commit-and-tag)
      DO_COMMIT=true
      DO_TAG=true
      shift
      ;;
    -*)
      echo "Unknown option: $1"
      usage
      ;;
    *)
      if [ -z "$NEW_VERSION" ]; then
        NEW_VERSION="$1"
      else
        echo "Error: Unexpected argument: $1"
        usage
      fi
      shift
      ;;
  esac
done

# Validate new version
if [ -z "$NEW_VERSION" ]; then
  echo -e "${RED}Error: New version is required${NC}"
  usage
fi

# Validate version format (semantic versioning)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?(\+[a-zA-Z0-9.]+)?$ ]]; then
  echo -e "${RED}Error: Invalid version format: $NEW_VERSION${NC}"
  echo -e "${YELLOW}Expected format: X.Y.Z (e.g., 0.1.2, 1.0.0, 2.1.3-beta.1)${NC}"
  exit 1
fi

# Extract current version
CURRENT_VERSION=$(grep -A 2 '^\[workspace.package\]' "$WORKSPACE_TOML" | grep '^version' | sed 's/version = "\(.*\)"/\1/')

if [ -z "$CURRENT_VERSION" ]; then
  echo -e "${RED}Error: Could not find workspace.package.version in $WORKSPACE_TOML${NC}"
  exit 1
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  RongJS Version Bump${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "Current version: ${YELLOW}${CURRENT_VERSION}${NC}"
echo -e "New version:     ${GREEN}${NEW_VERSION}${NC}"
echo ""
if [ "$DO_COMMIT" = true ] || [ "$DO_TAG" = true ]; then
  echo -e "Git operations:"
  [ "$DO_COMMIT" = true ] && echo -e "  - ${GREEN}✓${NC} Create commit"
  [ "$DO_TAG" = true ] && echo -e "  - ${GREEN}✓${NC} Create tag v${NEW_VERSION}"
else
  echo -e "${YELLOW}⚠️  DRY RUN MODE${NC}"
  echo -e "   Only updating Cargo.toml, no git operations"
  echo -e "   Use --commit-and-tag to commit and tag"
fi
echo ""

# Check for uncommitted changes
if [ "$DO_COMMIT" = true ] && ! git diff-index --quiet HEAD -- 2>/dev/null; then
  echo -e "${RED}Error: You have uncommitted changes${NC}"
  echo -e "${YELLOW}Please commit or stash your changes before bumping version${NC}"
  exit 1
fi

# Confirm
read -p "Proceed with version bump? (yes/no): " -r
echo
if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
  echo "Aborted."
  exit 0
fi

echo -e "${BLUE}Step 1: Updating workspace.package.version${NC}"

# Update workspace.package.version using Perl
perl -i -pe '
  BEGIN { $in_pkg = 0; }

  if (/^\[workspace\.package\]/) {
    $in_pkg = 1;
  } elsif (/^\[/ && $in_pkg) {
    $in_pkg = 0;
  }

  if ($in_pkg && /^version = /) {
    s/^version = ".*"/version = "'"$NEW_VERSION"'"/;
  }
' "$WORKSPACE_TOML"

echo -e "${GREEN}✓ Updated workspace.package.version to ${NEW_VERSION}${NC}"
echo ""

echo -e "${BLUE}Step 2: Syncing workspace.dependencies${NC}"

# Sync all workspace.dependencies versions
perl -i -pe '
  BEGIN { $version = "'"$NEW_VERSION"'"; $in_deps = 0; }

  if (/^\[workspace\.dependencies\]/) {
    $in_deps = 1;
  } elsif (/^\[/ && !/^\[workspace\.dependencies\]/) {
    $in_deps = 0;
  }

  if ($in_deps && /^rong[_a-z]* = \{/) {
    # Remove existing version attribute
    s/, *version = "[^"]*"//g;
    s/version = "[^"]*", *//g;

    # Add new version before closing brace
    s/\}$/, version = "$version" }/;
  }
' "$WORKSPACE_TOML"

echo -e "${GREEN}✓ Synced all workspace.dependencies to ${NEW_VERSION}${NC}"
echo ""

echo ""

if [ "$DO_COMMIT" = true ]; then
  echo -e "${BLUE}Step 3: Creating git commit${NC}"

  git add Cargo.toml
  git commit -m "chore: bump version to ${NEW_VERSION}"

  echo -e "${GREEN}✓ Created commit${NC}"
  echo ""
fi

if [ "$DO_TAG" = true ]; then
  echo -e "${BLUE}Step 4: Creating git tag${NC}"

  TAG_NAME="v${NEW_VERSION}"

  # Check if tag already exists
  if git rev-parse "$TAG_NAME" >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Tag ${TAG_NAME} already exists${NC}"
    read -p "Delete and recreate? (yes/no): " -r
    echo
    if [[ $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
      git tag -d "$TAG_NAME"
      git tag -a "$TAG_NAME" -m "Release ${NEW_VERSION}"
      echo -e "${GREEN}✓ Recreated tag ${TAG_NAME}${NC}"
    else
      echo -e "${YELLOW}Skipped tag creation${NC}"
    fi
  else
    git tag -a "$TAG_NAME" -m "Release ${NEW_VERSION}"
    echo -e "${GREEN}✓ Created tag ${TAG_NAME}${NC}"
  fi
  echo ""
fi

echo -e "${GREEN}Version bump complete.${NC}"
echo -e "Version bumped from ${YELLOW}${CURRENT_VERSION}${NC} to ${GREEN}${NEW_VERSION}${NC}"
echo ""

if [ "$DO_COMMIT" = true ] && [ "$DO_TAG" = true ]; then
  echo -e "${YELLOW}Next steps:${NC}"
  echo -e "  1. Review: ${BLUE}git show && git tag -l${NC}"
  echo -e "  2. Push: ${BLUE}git push && git push --tags${NC}"
  echo -e "  3. Publish: ${BLUE}./scripts/publish.sh${NC}"
elif [ "$DO_COMMIT" = true ]; then
  echo -e "${YELLOW}Next steps:${NC}"
  echo -e "  1. Review: ${BLUE}git show${NC}"
  echo -e "  2. Tag: ${BLUE}git tag -a v${NEW_VERSION} -m 'Release ${NEW_VERSION}'${NC}"
  echo -e "  3. Push: ${BLUE}git push && git push --tags${NC}"
  echo -e "  4. Publish: ${BLUE}./scripts/publish.sh${NC}"
elif [ "$DO_TAG" = true ]; then
  echo -e "${YELLOW}Next steps:${NC}"
  echo -e "  1. Review: ${BLUE}git diff Cargo.toml && git tag -l${NC}"
  echo -e "  2. Commit: ${BLUE}git add Cargo.toml && git commit -m 'chore: bump version to ${NEW_VERSION}'${NC}"
  echo -e "  3. Push: ${BLUE}git push && git push --tags${NC}"
  echo -e "  4. Publish: ${BLUE}./scripts/publish.sh${NC}"
else
  echo -e "${YELLOW}Next steps:${NC}"
  echo -e "  1. Review: ${BLUE}git diff Cargo.toml${NC}"
  echo -e "  2. Commit: ${BLUE}git add Cargo.toml && git commit -m 'chore: bump version to ${NEW_VERSION}'${NC}"
  echo -e "  3. Tag: ${BLUE}git tag -a v${NEW_VERSION} -m 'Release ${NEW_VERSION}'${NC}"
  echo -e "  4. Push: ${BLUE}git push && git push --tags${NC}"
  echo -e "  5. Publish: ${BLUE}./scripts/publish.sh${NC}"
  echo ""
  echo -e "${BLUE}💡 Tip: Use ${GREEN}./scripts/bump_version.sh ${NEW_VERSION} --commit-and-tag${BLUE} for one-step bump${NC}"
fi
