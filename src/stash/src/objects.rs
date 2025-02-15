// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The current types that are serialized in the Stash.

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/objects.rs"));
}

impl proto::Duration {
    pub const fn from_secs(secs: u64) -> proto::Duration {
        proto::Duration { secs, nanos: 0 }
    }
}

impl From<String> for proto::StringWrapper {
    fn from(value: String) -> Self {
        proto::StringWrapper { inner: value }
    }
}
