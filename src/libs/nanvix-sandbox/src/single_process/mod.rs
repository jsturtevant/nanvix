// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Single-process sandbox implementation.
//!
//! This module provides the default sandboxing functionality where Linux Daemon and User VM
//! instances are spawned as tasks within the same process.

//==================================================================================================
// Modules
//==================================================================================================

pub mod linuxd;
pub mod uservm;
