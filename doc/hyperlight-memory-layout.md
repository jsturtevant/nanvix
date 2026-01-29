# Hyperlight Memory Layout Integration

This document describes the memory layout challenges encountered when integrating Nanvix with Hyperlight and the solutions implemented.

## Background

The original workaround for Hyperlight's memory layout was introduced in [commit b170e299](https://github.com/nanvix/nanvix/commit/b170e2997c02e738d3bb06a763578d0db3c473a3#diff-30d631c85a493fa748c248d7f923b54c1b1c1f6bc56b7e5f569cca5f0d711ced), which added a padding hack to relocate the initrd. This document explains why that approach broke with larger kernels and how we fixed it.

## Problem Summary

Nanvix used hardcoded memory addresses that conflicted with Hyperlight's dynamic memory layout. When the kernel binary grew (e.g., adding filesystem support), Hyperlight shifted memory regions, causing:

1. **Initrd corruption** - The initrd was relocated to an address that overwrote the guest heap.
2. **Memory region misalignment** - Nanvix's KPOOL overlapped with Hyperlight's PEB and buffers.

## Memory Layout Comparison

### Small Kernel (~1.8MB code) - Old Layout (Broken)

```
Address        Hyperlight Places        Nanvix Expected
─────────────────────────────────────────────────────────
0x400000       PEB                      KPOOL_BASE ✗
0x401000       Host Function Defs       KPOOL
0x402000       Input Buffer             KPOOL
0x406000       Output Buffer            KPOOL
0x400000       Guest Heap (4MB)         KPOOL
0x800000       Guard + Stack            KPOOL_END
0x802000       Init Data                ✓ (worked by coincidence)
```

### Large Kernel (~4MB code) - Old Layout (Broken)

```
Address        Hyperlight Places        Nanvix Expected
─────────────────────────────────────────────────────────
0x400000       PEB                      KPOOL_BASE ✗
0x401000       Host Function Defs       KPOOL
0x800000       Guest Heap (4MB)         KPOOL_END
0xC00000       Guard Page               —
0xC01000       Guest Stack              —
0xC02000       Init Data                ✗ DEFAULT_INITRD_BASE=0x802000
```

**The issue:** Hyperlight's 4MB heap alignment pushed `init_data` from `0x802000` to `0xC02000`. The hardcoded `DEFAULT_INITRD_BASE` caused a 4MB backwards copy that corrupted the guest heap.

### New Layout (Fixed)

After the fix, Nanvix uses PEB-provided addresses instead of hardcoded constants:

```
Address        Region                   Source
─────────────────────────────────────────────────────────
0x001000       Guest Code               Kernel binary
   ...            ...                      ...
0x400000       PEB                      peb_base (from __KERNEL_END aligned)
0x401000       Host Function Defs       peb.host_function_definitions.ptr
0x402000       Input Buffer             peb.input_stack.ptr
0x406000       Output Buffer            peb.output_stack.ptr
0x800000       KPOOL / Guest Heap       peb.guest_heap.ptr (KPOOL_BASE for hyperlight)
0xC00000       Guard Page               (heap + heap_size)
0xC01000       Guest Stack              peb.guest_stack.min_user_stack_address
0xC02000       Init Data (Initrd)       peb.init_data.ptr ← NOW USED DIRECTLY
   ...         Initrd Payload           peb.init_data.ptr + 8 (after size header)
0xFA14000      FS Manifest              peb.guest_fs_manifest.ptr
0xFA15000      FS Region                peb.guest_fs_region.ptr
```

**Key difference:** The initrd stays at `peb.init_data.ptr` instead of being copied to a hardcoded address.

## How Hyperlight Calculates Layout

```
code_size         = kernel binary size (rounded to 4KB)
peb_offset        = align_up(code_size, 4KB)
buffers           = peb + host_func_defs + input + output
guest_heap_offset = align_up(buffers_end, 4MB)      ← 4MB alignment!
guard_page        = heap + heap_size
stack             = guard + 4KB
init_data         = stack + stack_size              ← varies with code size
```

## Changes Made

### 1. Fixed Initrd Relocation (`src/kernel/src/hal/platform/hyperlight/mod.rs`)

**Before:** Relocated initrd to hardcoded `DEFAULT_INITRD_BASE` (0x802000).

**After:** Relocate within the `init_data` region using PEB-provided address.

```rust
// Before (broken)
let dst_ptr: *mut u8 = ::config::hyperlight::DEFAULT_INITRD_BASE as *mut u8;

// After (fixed)
let dst_ptr: *mut u8 = init_data_start as *mut u8;
```

The 8-byte relocation now shifts the ELF to skip the size header while staying within the correct memory region.

### 2. Adjusted KPOOL_BASE for Hyperlight (`src/libs/config/src/lib.rs`)

**Before:** `KPOOL_BASE_RAW = 0x00400000` (conflicts with PEB).

**After:** Conditional compilation sets `KPOOL_BASE_RAW = 0x00800000` for Hyperlight builds.

```rust
cfg_if::cfg_if! {
    if #[cfg(feature = "hyperlight")] {
        pub const KPOOL_BASE_RAW: usize = 0x00800000;
    } else {
        pub const KPOOL_BASE_RAW: usize = 0x00400000;
    }
}
```

## Key Insight

The PEB (Process Environment Block) contains all memory region addresses. Rather than calculating offsets from `__KERNEL_END` or using hardcoded constants, **always use the PEB-provided pointers**:

- `peb.init_data.ptr` / `peb.init_data.size`
- `peb.guest_heap.ptr` / `peb.guest_heap.size`
- `peb.input_stack.ptr` / `peb.output_stack.ptr`
- `peb.guest_fs_region.ptr` / `peb.guest_fs_manifest.ptr`

This ensures compatibility regardless of kernel size or Hyperlight's internal layout decisions.

## Debugging Tips

Enable trace logging to see the actual memory layout:

```bash
./z build -- BUILD_OPT=no MACHINE=hyperlight LOG_LEVEL=trace && \
RUST_LOG=trace ./bin/nanvixd.elf -ramfs README.md -- bin/hello-c.elf
```

Look for these log entries to verify addresses:

```
peb.init_data.ptr=0x...
peb.guest_heap.ptr=0x...
parse_initrd_image(): initrd relocated from 0x... to 0x...
```