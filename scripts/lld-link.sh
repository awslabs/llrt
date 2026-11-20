#!/bin/bash
# Link with lld for --icf=all and borrow zig's C++ runtime for snmalloc.
# Env: LLRT_LLD, LLRT_ZIG_TARGET, LLRT_ZIG_SYSROOT (set by the Makefile).
set -euo pipefail

sysroot=$LLRT_ZIG_SYSROOT
mkdir -p "$sysroot"

# zig's archive paths are content-addressed; hashing them plus the version
# invalidates the marker when zig changes.
printf 'int main(void) { return 0; }\n' >"$sysroot/probe.cc"
link_line=$(zig c++ -target "$LLRT_ZIG_TARGET" -static -v \
	-o "$sysroot/probe" "$sysroot/probe.cc" 2>&1 | grep '^ld\.lld ' | tail -1)
rm -f "$sysroot/probe" "$sysroot/probe.cc"
if [ -z "$link_line" ]; then
	echo "error: no linker command from zig for $LLRT_ZIG_TARGET" >&2
	exit 1
fi

signature=$(printf '%s\n%s\n' "$(zig version)" "$link_line")
if command -v sha256sum >/dev/null 2>&1; then
	signature=$(printf '%s' "$signature" | sha256sum | cut -c1-16)
else
	signature=$(printf '%s' "$signature" | shasum -a 256 | cut -c1-16)
fi
ready="$sysroot/.ready-$signature"

if [ ! -f "$ready" ]; then
	rm -f "$sysroot"/.ready-*

	# Expose zig's archives under the -l names rustc asks for.
	for dir in $(echo "$link_line" | tr ' ' '\n' | grep -E '^/.*\.a$' | sed 's|/[^/]*$||' | sort -u); do
		for lib in "$dir"/*.a; do
			ln -sf "$lib" "$sysroot/$(basename "$lib")"
		done
	done
	ln -sf libc++.a "$sysroot/libstdc++.a"
	ln -sf libzigc.a "$sysroot/libatomic.a"
	ln -sf libzigc.a "$sysroot/libgcc_s.a"
	ln -sf libcompiler_rt.a "$sysroot/libgcc.a"
	for name in m pthread dl rt util resolv; do
		ln -sf libc.a "$sysroot/lib$name.a"
	done
	touch "$ready"
fi

args=(--icf=all)
skip_next=0
for arg in "$@"; do
	if [ "$skip_next" = 1 ]; then
		skip_next=0
		continue
	fi
	case $arg in
	-flavor)
		skip_next=1
		;;
	-nodefaultlibs|-nostartfiles|-no-pie)
		;;
	*)
		args+=("$arg")
		;;
	esac
done

# -L last so rustc's libc wins for -lc.
args+=(-L"$sysroot" -lc++abi)
exec "$LLRT_LLD" "${args[@]}"
