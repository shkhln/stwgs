#!/bin/sh
TARGET="$(realpath "$0")"
env RUST_BACKTRACE=1 \
  VK_INSTANCE_LAYERS=VK_LAYER_STWGS_overlay:VK_LAYER_MESA_overlay \
  VK_LAYER_PATH="${TARGET%/*}":/usr/local/share/vulkan/explicit_layer.d \
  vkcube-xcb "$@"
