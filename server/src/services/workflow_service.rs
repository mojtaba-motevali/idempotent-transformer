use std::sync::Arc;

use chrono::Utc;
use tracing::instrument;

use crate::{
    error::WorkflowError,
    repositories::{
        checkpoints::CheckpointRepository,
        workflows::WorkflowRepository,
        workflows_fencing_tokens::FencingTokenRepository,
    },
    schema::workflow::Workflow,
    storage::{
        command::{WorkflowCommand, WorkflowResponse},
        type_config::{NodeId, TypeConfig},
    },
};

pub struct CreateWorkflowOutput {
    pub fencing_token: i64,
}

#[derive(Debug)]
pub struct FinishWorkflowInput {
    pub workflow_id: String,
    pub fencing_token: i64,
    pub expire_after: i64,
}

pub struct WorkflowService {
    workflows: Arc<WorkflowRepository>,
    fencing_tokens: Arc<FencingTokenRepository>,
    checkpoints: Arc<CheckpointRepository>,
    raft: Arc<openraft::Raft<TypeConfig>>,
    local_node_id: NodeId,
}

impl WorkflowService {
    pub fn new(
        workflows: Arc<WorkflowRepository>,
        fencing_tokens: Arc<FencingTokenRepository>,
        checkpoints: Arc<CheckpointRepository>,
        raft: Arc<openraft::Raft<TypeConfig>>,
        local_node_id: NodeId,
    ) -> Self {
        Self {
            workflows,
            fencing_tokens,
            checkpoints,
            raft,
            local_node_id,
        }
    }

    pub async fn is_leader(&self) -> bool {
        self.raft.current_leader().await == Some(self.local_node_id)
    }

    #[instrument(skip(self), fields(workflow_id = %workflow_id), err)]
    pub async fn create(
        &self,
        workflow_id: &str,
        name: Option<&str>,
    ) -> Result<CreateWorkflowOutput, WorkflowError> {
        let created_at = Utc::now().timestamp_millis();

        let (workflow_res, token_res) = tokio::join!(
            self.raft.client_write(WorkflowCommand::CreateOrGetWorkflow {
                workflow_id: workflow_id.to_owned(),
                name: name.map(str::to_owned),
                created_at,
            }),
            self.raft.client_write(WorkflowCommand::IncrementFencingToken {
                workflow_id: workflow_id.to_owned(),
            }),
        );

        workflow_res.map_err(|e| WorkflowError::Raft(e.to_string()))?;
        let token_entry = token_res.map_err(|e| WorkflowError::Raft(e.to_string()))?;

        let fencing_token = match token_entry.data {
            WorkflowResponse::FencingToken(t) => t,
            other => {
                return Err(WorkflowError::Raft(format!(
                    "unexpected response variant: {other:?}"
                )));
            }
        };

        Ok(CreateWorkflowOutput { fencing_token })
    }

    #[instrument(skip(self), fields(workflow_id = %input.workflow_id, fencing_token = input.fencing_token), err)]
    pub async fn finish(&self, input: FinishWorkflowInput) -> Result<(), WorkflowError> {
        let stored = self
            .fencing_tokens
            .get(&input.workflow_id)
            .await?
            .ok_or_else(|| WorkflowError::FencingTokenNotFound {
                workflow_id: input.workflow_id.clone(),
            })?;

        if stored > input.fencing_token {
            return Err(WorkflowError::FencingTokenExpired {
                stored,
                received: input.fencing_token,
            });
        }

        let now = Utc::now().timestamp_millis();
        let expire_at = now + input.expire_after;

        self.raft
            .client_write(WorkflowCommand::CompleteWorkflow {
                workflow_id: input.workflow_id,
                expire_at,
                completed_at: now,
            })
            .await
            .map_err(|e| WorkflowError::Raft(e.to_string()))?;

        Ok(())
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn status(&self, workflow_id: &str) -> Result<Workflow, WorkflowError> {
        self.workflows
            .get_by_id(workflow_id)
            .await?
            .ok_or_else(|| WorkflowError::NotFound {
                id: workflow_id.to_owned(),
            })
    }

    #[instrument(skip(self), err)]
    pub async fn cleanup_expired(&self) -> Result<(), WorkflowError> {
        if !self.is_leader().await {
            return Ok(());
        }

        let before_ms = Utc::now().timestamp_millis();

        let (tokens_res, checkpoints_res) = tokio::join!(
            self.fencing_tokens.delete_for_completed(before_ms),
            self.checkpoints.delete_for_completed(before_ms),
        );
        tokens_res?;
        checkpoints_res?;

        let n = self.workflows.delete_expired(before_ms).await?;
        tracing::debug!(deleted = n, "expired workflows removed");

        Ok(())
    }
}
