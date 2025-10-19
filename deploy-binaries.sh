#!/usr/bin/env bash

nix build https://github.com/neosam/intenturly/archive/$1.zip#backend-oidc
nix-copy-closure --to shifty.nebenan-unverpackt.de result
nix build https://github.com/neosam/intenturly/archive/$1.zip#frontend
nix-copy-closure --to shifty.nebenan-unverpackt.de result
