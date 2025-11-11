#!/usr/bin/env bash

PROFILE="debug"
PROFILE_FLAG=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --release|release)
            PROFILE="release"
            PROFILE_FLAG="--release"
            shift
            ;;
        *)
            echo "Unknown argument: $1"
            echo "Usage: $0 [release|--release]"
            exit 1
            ;;
    esac
done

cp ../../../crates/audio_stream/src/wasm/audio_stream_worklet.js public/

cargo build \
  -p freeverb_module \
  --target wasm32-unknown-unknown \
  $PROFILE_FLAG
wasm-bindgen \
  --out-dir public \
  --out-name freeverb \
  --no-modules --no-typescript \
  ../../../target/wasm32-unknown-unknown/$PROFILE/freeverb_module.wasm

cargo build \
  -p web_egui \
  --target wasm32-unknown-unknown \
  $PROFILE_FLAG
wasm-bindgen \
  --out-dir public \
  --out-name web_egui \
  --no-modules --no-typescript \
  ../../../target/wasm32-unknown-unknown/$PROFILE/web_egui.wasm
