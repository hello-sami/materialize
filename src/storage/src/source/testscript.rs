// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use std::time::Duration;

use timely::scheduling::SyncActivator;
use tokio::time::sleep;

use mz_expr::PartitionId;
use mz_repr::GlobalId;
use mz_storage_client::types::connections::ConnectionContext;
use mz_storage_client::types::sources::encoding::SourceDataEncoding;
use mz_storage_client::types::sources::{MzOffset, TestScriptSourceConnection};

use crate::source::commit::LogCommitter;
use crate::source::types::SourceConnectionBuilder;
use crate::source::{SourceMessage, SourceMessageType, SourceReader};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(tag = "command")]
#[serde(rename_all = "lowercase")]
pub enum ScriptCommand {
    /// Emit a value (and possibly a key, which may be required
    /// by the envelope), at an offset.
    Emit {
        key: Option<String>,
        value: String,
        offset: u64,
    },
    /// Terminate the source. Commands after this command
    /// are ignored. Absence of this command causes the source
    /// to pend forever after all commands are processed, like
    /// a kafka source for a topic with no new messages.
    Terminate,
}

struct Script {
    commands: std::vec::IntoIter<ScriptCommand>,
}

pub struct TestScriptSourceReader {
    script: Script,
}

impl SourceConnectionBuilder for TestScriptSourceConnection {
    type Reader = TestScriptSourceReader;
    type OffsetCommitter = LogCommitter;

    fn into_reader(
        self,
        _source_name: String,
        source_id: GlobalId,
        worker_id: usize,
        worker_count: usize,
        _consumer_activator: SyncActivator,
        _restored_offsets: Vec<(PartitionId, Option<MzOffset>)>,
        _encoding: SourceDataEncoding,
        _metrics: crate::source::metrics::SourceBaseMetrics,
        _connection_context: ConnectionContext,
    ) -> Result<(Self::Reader, Self::OffsetCommitter), anyhow::Error> {
        let commands: Vec<ScriptCommand> = serde_json::from_str(&self.desc_json)?;
        Ok((
            TestScriptSourceReader {
                script: Script {
                    commands: commands.into_iter(),
                },
            },
            LogCommitter {
                source_id,
                worker_id,
                worker_count,
            },
        ))
    }
}

#[async_trait::async_trait(?Send)]
impl SourceReader for TestScriptSourceReader {
    type Key = Option<Vec<u8>>;
    type Value = Option<Vec<u8>>;
    type Time = MzOffset;
    type Diff = ();

    async fn next(
        &mut self,
        timestamp_granularity: Duration,
    ) -> Option<SourceMessageType<Self::Key, Self::Value, Self::Diff>> {
        if let Some(command) = self.script.commands.next() {
            match command {
                ScriptCommand::Terminate => {
                    return None;
                }
                ScriptCommand::Emit { key, value, offset } => {
                    // For now we only support `Finalized` messages
                    let msg = Ok(SourceMessage {
                        // For now, we only support single-output, single partition
                        output: 0,
                        upstream_time_millis: None,
                        key: key.map(|k| k.into_bytes()),
                        value: Some(value.into_bytes()),
                        headers: None,
                    });
                    let ts = (PartitionId::None, MzOffset::from(offset));
                    return Some(SourceMessageType::Finalized(msg, ts, ()));
                }
            }
        } else {
            loop {
                // sleep continuously
                sleep(timestamp_granularity).await;
            }
        }
    }
}
