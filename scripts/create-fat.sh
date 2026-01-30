#!/bin/bash
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Creates a FAT image containing specified files or directories.
#
# Usage:
#   ./scripts/create-fat.sh <source> <guest_path> [output]
#
# Arguments:
#   source     - Host file or directory to bundle
#   guest_path - Path where content will appear in the FAT image
#   output     - Output FAT image path (default: lib/fat/<basename>.fat)
#
# Requirements:
#   - dosfstools (provides mkfs.fat)
#   - mtools (provides mcopy, mmd)
#
# Examples:
#   # Bundle a single file
#   ./scripts/create-fat.sh README.md /README.md
#
#   # Bundle Python stdlib
#   ./scripts/create-fat.sh sysroot-debug/lib/python3.12 /usr/lib/python3.12
#
#   # Bundle with custom output path
#   ./scripts/create-fat.sh sysroot-debug/lib/python3.12 /usr/lib/python3.12 my-python.fat

set -e

# Parse arguments.
if [ $# -lt 2 ]; then
    echo "Usage: $0 <source> <guest_path> [output]" >&2
    echo "" >&2
    echo "Arguments:" >&2
    echo "  source     - Host file or directory to bundle" >&2
    echo "  guest_path - Path where content will appear in the FAT image" >&2
    echo "  output     - Output FAT image path (default: lib/fat/<basename>.fat)" >&2
    echo "" >&2
    echo "Examples:" >&2
    echo "  $0 README.md /README.md" >&2
    echo "  $0 sysroot-debug/lib/python3.12 /usr/lib/python3.12" >&2
    exit 1
fi

SOURCE="$1"
GUEST_PATH="$2"
BASENAME=$(basename "$SOURCE")
OUTPUT="${3:-lib/fat/${BASENAME}.fat}"

# Validate source exists.
if [ ! -e "$SOURCE" ]; then
    echo "Error: Source not found: $SOURCE" >&2
    exit 1
fi

# Validate guest path starts with /.
if [[ ! "$GUEST_PATH" =~ ^/ ]]; then
    echo "Error: Guest path must be absolute (start with /): $GUEST_PATH" >&2
    exit 1
fi

# Check for required tools.
for tool in mkfs.fat mcopy mmd; do
    if ! command -v "$tool" &> /dev/null; then
        echo "Error: Required tool '$tool' not found." >&2
        echo "Install with: sudo apt-get install dosfstools mtools" >&2
        exit 1
    fi
done

# Create output directory if it doesn't exist.
mkdir -p "$(dirname "$OUTPUT")"

# Calculate size with 30% overhead for FAT metadata and fragmentation.
if [ -d "$SOURCE" ]; then
    CONTENT_SIZE=$(du -sb "$SOURCE" | cut -f1)
else
    CONTENT_SIZE=$(stat -c%s "$SOURCE")
fi

FAT_SIZE=$((CONTENT_SIZE * 130 / 100))
MIN_SIZE=$((1024 * 1024))  # 1MB minimum
if [ "$FAT_SIZE" -lt "$MIN_SIZE" ]; then
    FAT_SIZE=$MIN_SIZE
fi

# Round up to nearest 1MB for cleaner FAT formatting.
FAT_SIZE=$(( (FAT_SIZE + 1024*1024 - 1) / (1024*1024) * (1024*1024) ))

echo "Source:       $SOURCE"
echo "Guest path:   $GUEST_PATH"
echo "Content size: $(numfmt --to=iec "$CONTENT_SIZE" 2>/dev/null || echo "$CONTENT_SIZE bytes")"
echo "FAT image:    $OUTPUT"
echo "Image size:   $(numfmt --to=iec "$FAT_SIZE" 2>/dev/null || echo "$FAT_SIZE bytes")"

# Remove existing image if present.
if [ -f "$OUTPUT" ]; then
    echo "Removing existing image..."
    rm -f "$OUTPUT"
fi

# Create sparse file for the FAT image.
echo "Creating FAT image..."
truncate -s "$FAT_SIZE" "$OUTPUT"

# Format the FAT image.
# For small images (< 33MB), let mkfs.fat auto-select FAT12/FAT16.
# For larger images, use FAT32 explicitly.
# Note: FAT32 requires at least 65525 clusters. With 512-byte sectors and
# 1 sector/cluster, that's ~33MB minimum. Forcing FAT32 on smaller images
# creates invalid boot sectors that fail validation.
LABEL=$(echo "$BASENAME" | tr '[:lower:]' '[:upper:]' | tr -cd '[:alnum:]' | head -c 11)
FAT32_MIN_SIZE=$((33 * 1024 * 1024))  # 33MB
if [ "$FAT_SIZE" -ge "$FAT32_MIN_SIZE" ]; then
    echo "Using FAT32 format (image >= 33MB)..."
    mkfs.fat -F 32 -n "$LABEL" "$OUTPUT" > /dev/null
else
    echo "Using auto-detected FAT format (image < 33MB)..."
    mkfs.fat -n "$LABEL" "$OUTPUT" > /dev/null
fi

# Create parent directories in FAT image.
# Extract parent directory from guest path.
GUEST_PARENT=$(dirname "$GUEST_PATH")
if [ "$GUEST_PARENT" != "/" ] && [ -n "$GUEST_PARENT" ]; then
    echo "Creating directory structure..."
    # Build up path components and create each directory.
    CURRENT=""
    IFS='/' read -ra PARTS <<< "$GUEST_PARENT"
    for part in "${PARTS[@]}"; do
        if [ -n "$part" ]; then
            CURRENT="$CURRENT/$part"
            mmd -i "$OUTPUT" "::$CURRENT" 2>/dev/null || true
        fi
    done
fi

# Copy content into the FAT image.
echo "Copying content..."
if [ -d "$SOURCE" ]; then
    # For directories, copy contents into guest path.
    # First create the target directory.
    mmd -i "$OUTPUT" "::$GUEST_PATH" 2>/dev/null || true
    # Copy directory contents recursively.
    mcopy -i "$OUTPUT" -s "$SOURCE"/* "::$GUEST_PATH/"
else
    # For files, copy directly to guest path.
    mcopy -i "$OUTPUT" "$SOURCE" "::$GUEST_PATH"
fi

# Verify the copy succeeded.
echo "Verifying..."
FILE_COUNT=$(mdir -i "$OUTPUT" -/ :: 2>/dev/null | wc -l)
echo "Files in image: approximately $FILE_COUNT entries"

echo ""
echo "Done! FAT image created: $OUTPUT"
echo ""
echo "FAT image contains: $GUEST_PATH"
echo ""
echo "Usage in Rust (add_fat_image requires absolute path):"
echo "  let fat_path = std::fs::canonicalize(\"$OUTPUT\")?;"
echo "  let fs = HyperlightFSBuilder::new()"
echo "      .add_fat_image(&fat_path, \"/\")?"
echo "      .build()?;"
