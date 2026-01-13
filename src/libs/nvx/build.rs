// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(target_os, values(\"nanvix\"))");
}
