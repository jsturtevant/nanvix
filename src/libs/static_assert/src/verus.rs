// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(dead_code)]
#![allow(unused_imports)]

use vstd::prelude::*;

verus! {
    /// # Description
    /// Ensures Verus emits metadata for the `static_assert` crate while real
    /// specifications are under development.
    pub proof fn static_assert_verus_placeholder() {
        assume(true);
    }
}
