#!/usr/bin/env bash

# Build with mock_auth features (default)
nix-build ./default.nix --arg features '["mock_auth"]'