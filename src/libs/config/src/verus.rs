// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(dead_code)]
#![allow(unused_imports)]

use vstd::prelude::*;

verus! {
    /// # Description
    /// Emits a minimal proof so Verus generates `.vir` metadata for the
    /// configuration crate.
    pub proof fn config_verus_placeholder() {
        assume(true);
    }
}
