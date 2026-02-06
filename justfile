build:
    ./z build -- BUILD_OPT=yes LOG_LEVEL=info MACHINE=hyperlight all

fat32:
    ./scripts/create-fat.sh sysroot-debug/lib/python3.12 /python3.12 lib/fat/python3.12.fat

fat32-md:
    python3 -m compileall -q lib/packages/markdown-pkg/markdown
    ./scripts/create-fat.sh lib/packages/markdown-pkg /.local/lib/python3.12/site-packages lib/fat/markdown-pkg.fat

py:
    rm -rf logs ; RUST_LOG=trace ./bin/nanvixd.elf -fat lib/fat/python3.12.fat:/lib -mount src/user/hello-python/__main__.py:/__main__.py -- sysroot-debug/bin/python3 -B /__main__.py

md:
    rm -rf logs ; RUST_LOG=trace ./bin/nanvixd.elf -fat lib/fat/python3.12.fat:/lib -fat lib/fat/markdown-pkg.fat:/root -mount src/user/hello-markdown/__main__.py:/__main__.py -- sysroot-debug/bin/python3 -B /__main__.py