#!/bin/bash
set -e

if [ -z ${COMPILE_TARGET+x} ]; then
    COMPILE_TARGET="${CARGO_CFG_TARGET_ARCH}-${CARGO_CFG_TARGET_VENDOR}-${CARGO_CFG_TARGET_OS}-${CARGO_CFG_TARGET_ENV}"
fi

CC_TARGET=""

if [[ $COMPILE_TARGET == "x86_64-unknown-linux-gnu" ]]; then
    CC_TARGET="x86_64-linux-gnu"
elif [[ $COMPILE_TARGET == "aarch64-unknown-linux-gnu" ]]; then
    CC_TARGET="aarch64-linux-gnu"
elif [[ $COMPILE_TARGET == "x86_64-unknown-linux-musl" ]]; then
    CC_TARGET="x86_64-linux-musl"
elif [[ $COMPILE_TARGET == "aarch64-unknown-linux-musl" ]]; then
    CC_TARGET="aarch64-linux-musl"
fi

new_array=()
for value in "$@"
do
    [[ $value != *self-contained/*crt* ]] && new_array+=($value)
done

# echo "Build with target \"$CC_TARGET\""

# echo "====="
# echo zig cc -target $CC_TARGET "${new_array[@]}"
# echo "====="

zig cc -target $CC_TARGET "${new_array[@]}"