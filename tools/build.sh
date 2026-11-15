#!/bin/bash
# Build the recovery and the init program it hands the ramdisk over to.
set -e
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to the NDK r29}"
cargo ndk -t arm64-v8a --platform 30 build --release -p skyward -p skyward-system-init
echo "built target/aarch64-linux-android/release/skyward"
