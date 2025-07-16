import {
  CheckpointInput,
  CheckpointOutput,
  CompleteWorkflowInput,
  CompleteWorkflowOutput,
  LeaseCheckpointInput,
  LeaseCheckpointOutput,
  LeaseWorkflowInput,
  LeaseWorkflowOutput,
} from './interfaces';

export interface IdempotentRpcAdapter {
  /**
   * Leases a workflow.
   * @param input - The input for the lease workflow RPC.
   * @returns The output of the lease workflow RPC.
   */
  leaseWorkflow(input: LeaseWorkflowInput): Promise<LeaseWorkflowOutput>;
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
}

export * from './interfaces';
