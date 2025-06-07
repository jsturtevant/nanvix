// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(non_camel_case_types)]
#![allow(unexpected_cfgs)]

use ::anyhow::{
    Error,
    Result,
};
use ::std::alloc::{
    alloc,
    Layout,
};
use ::wasmtime::*;

struct MyState {
    name: String,
    count: usize,
}

fn main() -> Result<(), Error> {
    // First the wasm module needs to be compiled. This is done with a global
    // "compilation environment" within an `Engine`. Note that engines can be
    // further configured through `Config` if desired instead of using the
    // default like this is here.
    println!("Compiling module...");
    let engine = Engine::default();
    let module = Module::from_file(&engine, "hello.wat")?;

    // After a module is compiled we create a `Store` which will contain
    // instantiated modules and other items like host functions. A Store
    // contains an arbitrary piece of host information, and we use `MyState`
    // here.
    println!("Initializing...");
    let mut store = Store::new(
        &engine,
        MyState {
            name: "hello, world!".to_string(),
            count: 0,
        },
    );

    // Our wasm module we'll be instantiating requires one imported function.
    // the function takes no parameters and returns no results. We create a host
    // implementation of that function here, and the `caller` parameter here is
    // used to get access to our original `MyState` value.
    println!("Creating callback...");
    let hello_func = Func::wrap(&mut store, |mut caller: Caller<'_, MyState>| {
        println!("Calling back...");
        println!("> {}", caller.data().name);
        caller.data_mut().count += 1;
    });

    // Once we've got that all set up we can then move to the instantiation
    // phase, pairing together a compiled module as well as a set of imports.
    // Note that this is where the wasm `start` function, if any, would run.
    println!("Instantiating module...");
    let imports = [hello_func.into()];
    let instance = Instance::new(&mut store, &module, &imports)?;

    // Next we poke around a bit to extract the `run` function from the module.
    println!("Extracting export...");
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    // And last but not least we can call it!
    println!("Calling export...");
    run.call(&mut store, ())?;

    println!("Done.");
    Ok(())
}

pub const WASMTIME_PROT_READ: u32 = 1 << 0;
pub const WASMTIME_PROT_WRITE: u32 = 1 << 1;
pub const WASMTIME_PROT_EXEC: u32 = 1 << 2;

pub use WASMTIME_PROT_EXEC as PROT_EXEC;
pub use WASMTIME_PROT_READ as PROT_READ;
pub use WASMTIME_PROT_WRITE as PROT_WRITE;

pub type wasmtime_trap_handler_t =
    extern "C" fn(ip: usize, fp: usize, has_faulting_addr: bool, faulting_addr: usize);

pub struct wasmtime_memory_image {
    ptr: *mut u8,
    len: usize,
}

#[unsafe(no_mangle)]
pub unsafe fn wasmtime_mmap_new(size: usize, _prot_flags: u32, ret: &mut *mut u8) -> i32 {
    // eprintln!("wasmtime_mmap_new(): size={size:?}, prot_flags={prot_flags:?}");
    if size == 0 {
        *ret = std::ptr::null_mut();
        return -1; // Invalid size
    }
    let layout = match Layout::from_size_align(size, 4096) {
        Ok(l) => l,
        Err(_) => {
            *ret = std::ptr::null_mut();
            return -1; // Allocation failed
        },
    };
    let ptr = alloc(layout);
    if ptr.is_null() {
        *ret = std::ptr::null_mut();
        return -1; // Allocation failed
    }
    *ret = ptr;
    0
}

#[unsafe(no_mangle)]
pub unsafe fn wasmtime_mmap_remap(_addr: *mut u8, _size: usize, _prot_flags: u32) -> i32 {
    // eprintln!("wasmtime_mmap_remap(): addr={addr:?}, size={size:?}, prot_flags={prot_flags:?}");
    // Dummy: always succeed
    0
}

#[unsafe(no_mangle)]
pub unsafe fn wasmtime_munmap(_ptr: *mut u8, _size: usize) -> i32 {
    // eprintln!("wasmtime_munmap(): ptr={:?}, size={size:?}", ptr);
    // Dummy: always succeed
    0
}

#[unsafe(no_mangle)]
pub unsafe fn wasmtime_mprotect(_ptr: *mut u8, _size: usize, _prot_flags: u32) -> i32 {
    // eprintln!("wasmtime_mprotect(): ptr={ptr:?}, size={size:?}, prot_flags={prot_flags:?}");
    // Dummy: always succeed
    0
}

#[unsafe(no_mangle)]
pub fn wasmtime_page_size() -> usize {
    // eprintln!("wasmtime_page_size() called");
    // Rust stdlib does not expose page size, so use a common default.
    4096
}

#[unsafe(no_mangle)]
pub unsafe fn wasmtime_setjmp(
    _jmp_buf: *mut *const u8,
    callback: extern "C" fn(*mut u8, *mut u8) -> bool,
    payload: *mut u8,
    callee: *mut u8,
) -> bool {
    // eprintln!(
    //     "wasmtime_setjmp() called with jmp_buf={:?}, payload={:?}, callee={:?}",
    //     jmp_buf, payload, callee
    // );
    // Dummy: just call the callback
    callback(payload, callee)
}

#[unsafe(no_mangle)]
pub unsafe fn wasmtime_longjmp(_jmp_buf: *const u8) -> ! {
    // eprintln!("wasmtime_longjmp() called with jmp_buf={:?}", jmp_buf);
    // Dummy: abort the process
    std::process::abort()
}

#[unsafe(no_mangle)]
pub unsafe fn wasmtime_init_traps(_handler: wasmtime_trap_handler_t) -> i32 {
    // Dummy: always succeed
    0
}

#[unsafe(no_mangle)]
pub unsafe fn wasmtime_memory_image_new(
    ptr: *const u8,
    len: usize,
    ret: &mut *mut wasmtime_memory_image,
) -> i32 {
    // eprintln!("wasmtime_memory_image_new(): ptr={:?}, len={len:?}", ptr);
    if ptr.is_null() || len == 0 {
        *ret = std::ptr::null_mut();
        return -1;
    }

    let layout = match Layout::array::<u8>(len) {
        Ok(l) => l,
        Err(_) => {
            *ret = std::ptr::null_mut();
            return -1;
        },
    };

    let dst = alloc(layout);
    if dst.is_null() {
        *ret = std::ptr::null_mut();
        return -1;
    }

    std::ptr::copy_nonoverlapping(ptr, dst, len);

    let layout2 = Layout::new::<wasmtime_memory_image>();

    let image = std::alloc::alloc(layout2) as *mut wasmtime_memory_image;
    if image.is_null() {
        std::alloc::dealloc(dst, layout);
        *ret = std::ptr::null_mut();
        return -1;
    }

    (*image).ptr = dst;
    (*image).len = len;

    *ret = image;
    0
}

#[unsafe(no_mangle)]
pub unsafe fn wasmtime_memory_image_map_at(
    _image: *mut wasmtime_memory_image,
    _addr: *mut u8,
    _len: usize,
) -> i32 {
    // eprintln!("wasmtime_memory_image_map_at(): image={:?}, addr={:?}, len={len:?}", image, addr);
    // Dummy: always succeed
    0
}

#[unsafe(no_mangle)]
pub unsafe fn wasmtime_memory_image_free(image: *mut wasmtime_memory_image) {
    // eprintln!("wasmtime_memory_image_free(): image={:?}", image);
    if image.is_null() {
        return;
    }

    let ptr = (*image).ptr;
    let len = (*image).len;
    let layout = match Layout::array::<u8>(len) {
        Ok(l) => l,
        Err(_) => return, // If layout creation fails, just return
    };
    std::alloc::dealloc(ptr, layout);
    let layout2 = Layout::new::<wasmtime_memory_image>();
    std::alloc::dealloc(image as *mut u8, layout2);
}

static mut WASMTIME_TLS: *mut u8 = std::ptr::null_mut();

#[unsafe(no_mangle)]
pub fn wasmtime_tls_get() -> *mut u8 {
    // eprintln!("wasmtime_tls_get()");
    unsafe { WASMTIME_TLS }
}

#[unsafe(no_mangle)]
pub fn wasmtime_tls_set(ptr: *mut u8) {
    // eprintln!("wasmtime_tls_set(): ptr={:?}", ptr);
    unsafe { WASMTIME_TLS = ptr }
}
