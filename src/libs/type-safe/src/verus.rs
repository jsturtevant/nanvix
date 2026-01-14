// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(dead_code)]
#![allow(unused_imports)]

use vstd::prelude::*;

verus! {
    /// # Description
    /// Minimal proof artifact so Verus can emit metadata for the `type-safe`
    /// crate until full specifications are available.
    pub proof fn type_safe_verus_placeholder() {
        assume(true);
    }
}
