#!/usr/bin/env bash

# Build with OIDC features
nix-build ./default.nix --arg features '["oidc"]'