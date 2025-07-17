import { credentials, ServiceError, VerifyOptions } from '@grpc/grpc-js';
import {
  CheckpointInput,
  CheckpointLeasedByOtherWorkerException,
  CheckpointOutput,
  CompleteWorkflowInput,
  CompleteWorkflowOutput,
  FencingTokenExpiredException,
  IdempotentRpcAdapter,
  NestedWorkflowFencingTokenConflictException,
  LeaseCheckpointInput,
  LeaseCheckpointOutput,
  LeaseWorkflowInput,
  LeaseWorkflowOutput,
  LeaseTimeoutNotFoundException,
  FencingTokenNotFoundException,
} from '@idempotent-transformer/core';
import { WorkflowServiceImplClient } from '../gen/workflow_service_grpc_pb';
import {
  CheckPointRequest,
  CompleteWorkflowRequest,
  LeaseCheckpointRequest,
  LeaseWorkflowRequest,
} from '../gen/workflow_service_pb';

const exceptions = {
  ['checkpoint_leased_by_other_worker']: CheckpointLeasedByOtherWorkerException,
  ['fencing_token_expired']: FencingTokenExpiredException,
  ['nested_workflow_fencing_token_conflict']: NestedWorkflowFencingTokenConflictException,
  ['lease_timeout_not_found']: LeaseTimeoutNotFoundException,
  ['fencing_token_not_found']: FencingTokenNotFoundException,
};

const handleError = (err: ServiceError) => {
  const Exception = exceptions[err.details as keyof typeof exceptions];
  if (Exception) {
    return new Exception(err.details);
  }
  return err;
};
export class GrpcAdapter implements IdempotentRpcAdapter {
  private client: WorkflowServiceImplClient;
  constructor({
    host,
    port,
    tls,
  }: {
    host: string;
    port: number;
    tls?: {
      rootCerts?: Buffer | null;
      privateKey?: Buffer | null;
      certChain?: Buffer | null;
      verifyOptions?: VerifyOptions;
    };
  }) {
    this.client = new WorkflowServiceImplClient(
      `${host}:${port}`,
      tls
        ? credentials.createSsl(tls.rootCerts, tls.privateKey, tls.certChain, tls.verifyOptions)
        : credentials.createInsecure()
    );
  }

  async leaseWorkflow(input: LeaseWorkflowInput): Promise<LeaseWorkflowOutput> {
    const request = new LeaseWorkflowRequest();
    request.setWorkflowId(input.workflow_id);
    request.setIsNested(input.is_nested);
    request.setContextName(input.context_name);
    request.setPrefetchCheckpoints(input.prefetch_checkpoints);
    return new Promise((resolve, reject) => {
      this.client.lease_workflow(request, (err, response) => {
        if (err) {
          reject(handleError(err));
          return;
        }
        resolve({
          checkpoints: Object.fromEntries(response.getCheckpointsMap().entries()),
          fencing_token: response.getFencingToken(),
        });
      });
    });
  }

  async checkpoint(input: CheckpointInput): Promise<CheckpointOutput> {
    const request = new CheckPointRequest();
    request.setWorkflowId(input.workflow_id);
    request.setValue(input.value);
    request.setFencingToken(input.fencing_token);
    request.setWorkflowContextName(input.workflow_context_name);
    request.setCheckpointContextName(input.checkpoint_context_name);
    request.setPositionChecksum(input.position_checksum);
    return new Promise((resolve, reject) => {
      this.client.checkpoint(request, (err, response) => {
        if (err) {
          reject(handleError(err));
          return;
        }
        resolve({
          abort: response.getAbort(),
        });
      });
    });
  }

  async leaseCheckpoint(input: LeaseCheckpointInput): Promise<LeaseCheckpointOutput> {
    const request = new LeaseCheckpointRequest();
    request.setWorkflowId(input.workflow_id);
    request.setFencingToken(input.fencing_token);
    request.setLeaseTimeout(input.lease_timeout);
    request.setPositionChecksum(input.position_checksum);
    return new Promise((resolve, reject) => {
      this.client.lease_checkpoint(request, (err, response) => {
        if (err) {
          reject(handleError(err));
          return;
        }
        resolve({
          value: response.getValue() as Buffer,
        });
      });
    });
  }

  async completeWorkflow(input: CompleteWorkflowInput): Promise<CompleteWorkflowOutput> {
    const request = new CompleteWorkflowRequest();
    request.setWorkflowId(input.workflow_id);
    request.setFencingToken(input.fencing_token);
    request.setExpireAfter(input.expire_after);
    return new Promise((resolve, reject) => {
      this.client.complete_workflow(request, (err, response) => {
        if (err) {
          reject(handleError(err));
          return;
        }
        resolve({});
      });
    });
  }
}
