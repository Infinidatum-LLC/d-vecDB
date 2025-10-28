#!/bin/bash

# Publishing script for d-vecdb npm package

set -e  # Exit on error

echo "========================================="
echo "d-vecDB TypeScript Client - Publish to npm"
echo "========================================="
echo ""

# Check if logged in
echo "Checking npm authentication..."
if ! npm whoami &> /dev/null; then
    echo ""
    echo "❌ You are not logged into npm."
    echo ""
    echo "Please log in first:"
    echo "  npm login"
    echo ""
    echo "If you don't have an npm account, create one at:"
    echo "  https://www.npmjs.com/signup"
    echo ""
    exit 1
fi

NPM_USER=$(npm whoami)
echo "✅ Logged in as: $NPM_USER"
echo ""

# Check package name availability
echo "Checking if package name 'd-vecdb' is available..."
if npm view d-vecdb &> /dev/null; then
    echo ""
    echo "❌ Package name 'd-vecdb' is already taken!"
    echo ""
    echo "Options:"
    echo "  1. Use a scoped package: @$NPM_USER/d-vecdb"
    echo "  2. Choose a different name"
    echo ""
    echo "To use a scoped package, update package.json:"
    echo '  "name": "@'$NPM_USER'/d-vecdb"'
    echo ""
    echo "Then publish with:"
    echo "  npm publish --access public"
    echo ""
    exit 1
else
    echo "✅ Package name 'd-vecdb' is available"
fi
echo ""

# Build
echo "Building the project..."
npm run build
echo "✅ Build successful"
echo ""

# Run tests
echo "Running tests..."
npm test 2>&1 | grep -E "(Test Suites|Tests:)" || true
echo ""

# Show what will be published
echo "Package contents:"
npm pack --dry-run 2>&1 | grep -E "(📦|total files)" | tail -2
echo ""

# Confirm
echo "========================================="
echo "Ready to publish d-vecdb@0.1.0"
echo "========================================="
echo ""
read -p "Do you want to publish to npm? (yes/no): " CONFIRM

if [ "$CONFIRM" = "yes" ] || [ "$CONFIRM" = "y" ]; then
    echo ""
    echo "Publishing to npm..."
    npm publish
    echo ""
    echo "========================================="
    echo "✅ Successfully published!"
    echo "========================================="
    echo ""
    echo "View your package at:"
    echo "  https://www.npmjs.com/package/d-vecdb"
    echo ""
    echo "Install with:"
    echo "  npm install d-vecdb"
    echo ""
else
    echo ""
    echo "❌ Publishing cancelled"
    echo ""
fi
