// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(dead_code)]
#![allow(unused_imports)]

use vstd::prelude::*;

verus! {
    /// # Description
    /// Placeholder proof to keep Verus aware of the `sysalloc` crate while we
    /// bootstrap actual specifications.
    pub proof fn sysalloc_verus_placeholder() {
        assume(true);
    }
}
