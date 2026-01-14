// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(dead_code)]
#![allow(unused_imports)]

use vstd::prelude::*;

verus! {
    /// # Description
    /// Initializes the Verus metadata for the `sys` crate so downstream crates
    /// can import its specifications while we incrementally add real proofs.
    pub proof fn sys_verus_placeholder() {
        assume(true);
    }
}
