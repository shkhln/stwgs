#!/bin/sh
TARGET="$(realpath "$0")"
export RUST_BACKTRACE=1
export VK_INSTANCE_LAYERS=VK_LAYER_STWGS_overlay:VK_LAYER_MESA_overlay
export VK_LAYER_PATH="${TARGET%/*}":/usr/local/share/vulkan/explicit_layer.d
export STWGS_OVERLAY_WASM_MODULE="${TARGET%/*}/target/wasm32-unknown-unknown/debug/probes.wasm"
if [ "$#" -eq 0 ]; then
  vkcube-xcb
else
  "$@"
fi
