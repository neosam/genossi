#!/usr/bin/env bash

# Exit on any error
set -e

# Simple VM runner using nixos-rebuild
echo "Building VM with Inventurly service module..."
echo ""

# Build the VM and check for errors
if ! nixos-rebuild build-vm -I nixos-config=./test-simple-config.nix; then
    echo ""
    echo "ERROR: VM build failed!"
    echo "Check the error messages above."
    exit 1
fi

# Check if the VM binary exists
if [ ! -f "./result/bin/run-inventurly-test-vm" ]; then
    echo "ERROR: VM binary not found at ./result/bin/run-inventurly-test-vm"
    echo "Build may have succeeded but created a different binary name."
    exit 1
fi

echo ""
echo "Build successful!"
echo ""
echo "Starting VM to test Inventurly module..."
echo ""
echo "Service configured (using module.nix):"
echo "  inventurly-test: http://localhost:3000"
echo ""
echo "Once in VM, test with:"
echo "  systemctl status inventurly-test"
echo "  curl http://localhost:3000/api/openapi.json"
echo "  curl http://localhost:3000/swagger-ui/"
echo "  ls -la /var/lib/inventurly-test/"
echo ""
echo "Exit with: Ctrl+A, X"
echo ""

# Run the VM
./result/bin/run-inventurly-test-vm