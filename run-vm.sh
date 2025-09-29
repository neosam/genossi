#!/usr/bin/env bash

# Simple VM runner using nixos-rebuild
echo "Building VM with Inventurly service..."
nixos-rebuild build-vm -I nixos-config=./test-simple-config.nix

echo ""
echo "Starting VM with Inventurly service..."
echo ""
echo "Service configured:"
echo "  inventurly-simple: http://localhost:3000"
echo ""
echo "Once in VM, test with:"
echo "  systemctl status inventurly-simple"
echo "  curl http://localhost:3000/api/openapi.json"
echo "  curl http://localhost:3000/swagger-ui/"
echo ""
echo "Exit with: Ctrl+A, X"
echo ""

./result/bin/run-inventurly-test-vm