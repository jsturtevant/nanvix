# Adding Python Packages to Nanvix

This document describes how to add third-party Python packages to Nanvix for use
in the Hyperlight VM environment.

## Overview

Python packages can be added to Nanvix by:

1. Downloading the pure-Python wheel package
2. Extracting it to the site-packages directory
3. Pre-compiling bytecode (required for read-only FAT filesystem)
4. Recreating the Python FAT image

## Requirements

- Only pure-Python packages are supported (no C extensions)
- Packages must be compatible with Python 3.12
- All bytecode must be pre-compiled before creating the FAT image

## Step-by-Step Guide

### Step 1: Download the Package

Download the wheel file using pip. Use `--no-deps` if you want to manage
dependencies manually:

```bash
pip download --no-deps --only-binary=:all: markdown -d /tmp/wheels/
```

### Step 2: Extract to Staging Directory

Create a staging directory and extract the wheel:

```bash
mkdir -p lib/packages/markdown-pkg
unzip /tmp/wheels/markdown-*.whl -d lib/packages/markdown-pkg/
```

### Step 3: Install to site-packages

Copy the package to the Python site-packages directory:

```bash
cp -r lib/packages/markdown-pkg/markdown sysroot-debug/lib/python3.12/site-packages/
cp -r lib/packages/markdown-pkg/markdown-*.dist-info sysroot-debug/lib/python3.12/site-packages/
```

### Step 4: Pre-compile Bytecode

The FAT filesystem is read-only in the VM, so Python cannot write `.pyc` files
at runtime. Pre-compile all bytecode:

```bash
python3 -m compileall -q sysroot-debug/lib/python3.12/site-packages/markdown
```

### Step 5: Recreate the FAT Image

Recreate the Python FAT image with the new package included:

```bash
rm -f lib/fat/python3.12.fat
./scripts/create-fat.sh sysroot-debug/lib/python3.12 /lib/python3.12 lib/fat/python3.12.fat
```

**Important:** The guest path must be `/lib/python3.12` to match Python's
compiled-in paths.

## Running Python with Packages

Run a Python script with the FAT image mounted:

```bash
./bin/nanvixd.elf \
  -fat lib/fat/python3.12.fat:/ \
  -mount path/to/script.py:/__main__.py \
  -- sysroot-debug/bin/python3 /__main__.py
```

## Example: hello-markdown

The `src/user/hello-markdown/__main__.py` example demonstrates importing the
markdown package:

```python
import markdown

md_text = "# Hello\n\nThis is **bold**."
html = markdown.markdown(md_text)
print(html)
```

Run it with:

```bash
./bin/nanvixd.elf \
  -fat lib/fat/python3.12.fat:/ \
  -mount src/user/hello-markdown/__main__.py:/__main__.py \
  -- sysroot-debug/bin/python3 /__main__.py
```

## Troubleshooting

### "No module named 'encodings'" Error

The FAT image was created with an incorrect guest path. Ensure you use
`/lib/python3.12` as the guest path:

```bash
./scripts/create-fat.sh sysroot-debug/lib/python3.12 /lib/python3.12 lib/fat/python3.12.fat
```

### Import Hangs or Fails Silently

The bytecode was not pre-compiled. Run `python3 -m compileall` on the package
before creating the FAT image.

### Package Not Found

Ensure the package was copied to `sysroot-debug/lib/python3.12/site-packages/`
and the FAT image was recreated afterwards.
