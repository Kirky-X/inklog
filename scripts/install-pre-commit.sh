#!/bin/bash
#
# Install pre-commit hook for Inklog project
#
# This script installs the pre-commit hook by creating a symlink
# in the .git/hooks directory.
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PRE_COMMIT_SCRIPT="$SCRIPT_DIR/pre-commit"
GIT_HOOKS_DIR=".git/hooks"
PRE_COMMIT_LINK="$GIT_HOOKS_DIR/pre-commit"

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

echo "Inklog Pre-commit Hook Installer"
echo "=================================="
echo ""

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    print_error "Not in a git repository."
    echo "Please run this script from the project root directory."
    exit 1
fi

# Check if pre-commit script exists
if [ ! -f "$PRE_COMMIT_SCRIPT" ]; then
    print_error "Pre-commit script not found at: $PRE_COMMIT_SCRIPT"
    exit 1
fi

# Create git hooks directory if it doesn't exist
if [ ! -d "$GIT_HOOKS_DIR" ]; then
    print_warning "Creating git hooks directory..."
    mkdir -p "$GIT_HOOKS_DIR"
fi

# Remove existing pre-commit hook if it exists
if [ -e "$PRE_COMMIT_LINK" ] || [ -L "$PRE_COMMIT_LINK" ]; then
    print_warning "Removing existing pre-commit hook..."
    rm -f "$PRE_COMMIT_LINK"
fi

# Create symlink
if ln -s "../../scripts/pre-commit" "$PRE_COMMIT_LINK"; then
    print_success "Pre-commit hook installed successfully!"
else
    print_error "Failed to create symlink. Trying copy instead..."
    if cp "$PRE_COMMIT_SCRIPT" "$PRE_COMMIT_LINK"; then
        print_success "Pre-commit hook copied successfully!"
    else
        print_error "Failed to install pre-commit hook."
        exit 1
    fi
fi

echo ""
echo "The pre-commit hook will now run before each commit and perform:"
echo "  1. Code formatting check (cargo fmt)"
echo "  2. Clippy linting (cargo clippy)"
echo "  3. Compilation check (cargo check)"
echo "  4. Unit tests (cargo test --lib)"
echo ""
echo "To skip the pre-commit hook, use: git commit --no-verify"
echo ""

# Verify the hook is properly installed
if [ -f "$PRE_COMMIT_LINK" ] || [ -L "$PRE_COMMIT_LINK" ]; then
    print_success "Installation verified!"
    exit 0
else
    print_error "Installation verification failed!"
    exit 1
fi
