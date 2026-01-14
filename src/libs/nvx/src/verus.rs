// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(dead_code)]
#![allow(unused_imports)]

use vstd::prelude::*;

verus! {
    /// # Description
    /// Minimal proof artifact so Verus generates `.vir` metadata for the `nvx`
    /// runtime crate.
    pub proof fn nvx_verus_placeholder() {
        assume(true);
    }
}
