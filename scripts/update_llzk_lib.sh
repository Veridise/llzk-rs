#!/usr/bin/env bash

# Program: update_llzk_lib.sh
# Description: This script updates the llzk-lib submodule to the latest
# commit from its remote repository and updates the nix flake accordingly.
#
# Required Programs:
#   - git: For updating the git submodule
#   - nix: For building the nix flake
#
# Usage: ./scripts/update_llzk_lib.sh

set -e

git submodule update --remote llzk-sys/llzk-lib
nix flake update llzk
