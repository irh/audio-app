#!/usr/bin/env bash

# Based on https://github.com/mozilla/glean/blob/main/build-scripts/xc-universal-binary.sh

# This should be invoked from inside xcode, not manually
if [ "$#" -ne 1 ]
then
    echo "Usage (note: only call inside xcode!):"
    echo "Args: $*"
    echo "path-to-workspace-root/scripts/build-for-xcode.sh <BINARY_NAME>"
    exit 1
fi

BINARY_NAME=$1

set -eux

PATH=$PATH:$HOME/.cargo/bin

PROFILE=debug
RELFLAG=
if [[ "$CONFIGURATION" != "Debug" ]]; then
    PROFILE=release
    RELFLAG=--release
fi

set -euvx

# add homebrew bin path, as it's the most commonly used package manager on macOS
# this is needed for cmake on apple arm processors as it's not available by default
# TODO: Can this be removed now? (#80817)
export PATH="$PATH:/opt/homebrew/bin"

# Make Cargo output cache files in Xcode's directories
export CARGO_TARGET_DIR="$DERIVED_FILE_DIR/cargo"

# Xcode places `/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin`
# at the front of the path, with makes the build fail with `ld: library 'System' not found`, upstream issue:
# <https://github.com/rust-lang/rust/issues/80817>.
#
# Work around it by resetting the path, so that we use the system `cc`.
# TODO: Can this be removed now? (#80817)
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$PATH"

IS_SIMULATOR=0
if [ "${LLVM_TARGET_TRIPLE_SUFFIX-}" = "-simulator" ]; then
  IS_SIMULATOR=1
fi

EXECUTABLES=
for arch in $ARCHS; do
  case "$PLATFORM_NAME" in
    iphoneos)
      case "$arch" in
        x86_64)
          echo "Building for x86_64, but not a simulator build. What's going on?" >&2
          exit 2
          ;;

        arm64)
          TARGET=aarch64-apple-ios
      esac
      ;;

    iphonesimulator)
      case "$arch" in
        x86_64)
          export CFLAGS_x86_64_apple_ios="-target x86_64-apple-ios"
          TARGET=x86_64-apple-ios
          ;;

        arm64)
          TARGET=aarch64-apple-ios-sim
      esac
      ;;

    macosx)
      case "$arch" in
        x86_64)
          TARGET=x86_64-apple-darwin
          ;;

        arm64)
          TARGET=aarch64-apple-darwin
      esac
  esac

  cargo build $RELFLAG --target $TARGET --bin $BINARY_NAME

  # Collect the executables
  EXECUTABLES="$EXECUTABLES $DERIVED_FILE_DIR/cargo/$TARGET/$PROFILE/$BINARY_NAME"
done

# Combine executables, and place them at the output path expected by Xcode.
TARGET_EXE_PATH="$TARGET_BUILD_DIR/$EXECUTABLE_PATH"
# Ensure that the target exe's path exists, otherwise lipo fails on macOS.
mkdir -p "$(dirname $TARGET_EXE_PATH)"
lipo -create -output $TARGET_EXE_PATH $EXECUTABLES

