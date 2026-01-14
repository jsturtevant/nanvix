// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(dead_code)]
#![allow(unused_imports)]

use vstd::prelude::*;

verus! {
    /// # Description
    /// Minimal lemma used to bootstrap Verus metadata for the `error` crate.
    pub proof fn error_verus_placeholder() {
        assume(true);
    }
}
