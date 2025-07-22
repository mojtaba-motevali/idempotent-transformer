import {
  CheckpointInput,
  CheckpointOutput,
  CompleteWorkflowInput,
  CompleteWorkflowOutput,
  LeaseCheckpointInput,
  LeaseCheckpointOutput,
  StartWorkflowInput,
  StartWorkflowOutput,
  WorkflowStatusInput,
  WorkflowStatusOutput,
} from './interfaces';
import {
  ReleaseLeaseCheckpointInput,
  ReleaseLeaseCheckpointOutput,
} from './interfaces/release-leas-checkpoint.interface';

export interface IdempotentRpcAdapter {
  /**
   * Leases a workflow.
   * @param input - The input for the lease workflow RPC.
   * @returns The output of the lease workflow RPC.
   */
  startWorkflow(input: StartWorkflowInput): Promise<StartWorkflowOutput>;
  /**
   * Creates a checkpoint.
   * @param input - The input for the checkpoint RPC.
   * @returns The output of the checkpoint RPC.
   */
  checkpoint(input: CheckpointInput): Promise<CheckpointOutput>;
  /**
   * Leases a checkpoint.
   * @param input - The input for the lease checkpoint RPC.
   * @returns The output of the lease checkpoint RPC.
   */
  leaseCheckpoint(input: LeaseCheckpointInput): Promise<LeaseCheckpointOutput>;
  /**
   * Completes a workflow.
   * @param input - The input for the complete workflow RPC.
   * @returns The output of the complete workflow RPC.
   */
  completeWorkflow(input: CompleteWorkflowInput): Promise<CompleteWorkflowOutput>;

  /**
   * Finds a workflow.
   * @param input - The input for the find workflow RPC.
   * @returns The output of the find workflow RPC.
   */
  getWorkflowStatus(input: WorkflowStatusInput): Promise<WorkflowStatusOutput>;

  /**
   * Releases a leased checkpoint.
   * @param input - The input for the release leased checkpoint RPC.
   * @returns The output of the release leased checkpoint RPC.
   */
  releaseLeaseCheckpoint(input: ReleaseLeaseCheckpointInput): Promise<ReleaseLeaseCheckpointOutput>;
}

export * from './interfaces';
export * from './err-codes';
