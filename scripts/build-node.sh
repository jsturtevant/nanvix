#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

RULE=${1:-build}
NANVIX_HOME=${2:-$PWD}
TOOLCHAIN_DIR=${3:-$PWD/toolchain}
SYSROOT_DIR=${4:-$PWD/sysroot}

#===================================================================================================
# Global Variables
#===================================================================================================

OPT_DIR=$NANVIX_HOME/opt
NODE_HOME=$OPT_DIR/node

#===================================================================================================
# Clean
#===================================================================================================

clean() {
	make clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

distclean() {
	git clean -fdx
}

#===================================================================================================
# Build
#===================================================================================================

build() {
	export AR_host="ar"
	export CC_host="gcc -D_GNU_SOURCE -m32"
	export CXX_host="g++ -D_GNU_SOURCE -m32"
	export AR_target="$TOOLCHAIN_DIR/bin/i686-nanvix-ar"
	export CC_target="$TOOLCHAIN_DIR/bin/i686-nanvix-gcc"
	export CXX_target="$TOOLCHAIN_DIR/bin/i686-nanvix-g++"

	# Configure.
    ./configure \
        --cross-compiling \
		--dest-os nanvix \
		--dest-cpu x86 \
        --fully-static \
        --without-npm \
        --without-node-code-cache \
		--without-node-snapshot \
        --without-corepack \
        --without-ssl \
		--without-inspector \
		--without-amaro \
		--without-node-options \
		--without-intl \
		--no-ifaddrs \
	  	--v8-lite-mode \
		--disable-shared-readonly-heap \
        --verbose

	# Build.
	make node
}

#===================================================================================================

cd $NODE_HOME

# Save current environment variables.
OLD_AR=$AR
OLD_AS=$AS
OLD_CC=$CC
OLD_CXX=$CXX
OLD_CPP=$CPP
OLD_LD=$LD
OLD_CFLAGS=$CFLAGS
OLD_CXXFLAGS=$CXXFLAGS
OLD_LD_FLAGS=$LDFLAGS
OLD_LIBC=$LIBC
OLD_LIBM=$LIBM

# Unset variables that might interfere with the build process.
unset AR
unset AS
unset CC
unset CXX
unset CPP
unset LD
unset CFLAGS
unset CXXFLAGS
unset LDFLAGS
unset LIBC
unset LIBM

case $RULE in
	build)
		build
		;;
	clean)
		clean
		;;
	distclean)
		distclean
		;;
esac

# Restore original environment variables.
export AR=$OLD_AR
export AS=$OLD_AS
export CC=$OLD_CC
export CXX=$OLD_CXX
export CPP=$OLD_CPP
export LD=$OLD_LD
export CFLAGS=$OLD_CFLAGS
export CXXFLAGS=$OLD_CXXFLAGS
export LDFLAGS=$OLD_LD_FLAGS
export LIBC=$OLD_LIBC
export LIBM=$OLD_LIBM
