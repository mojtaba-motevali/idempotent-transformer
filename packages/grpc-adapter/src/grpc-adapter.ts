import { credentials, ServiceError, VerifyOptions } from '@grpc/grpc-js';
import {
  CheckpointInput,
  CheckpointOutput,
  CompleteWorkflowInput,
  CompleteWorkflowOutput,
  IdempotentRpcAdapter,
  LeaseCheckpointInput,
  LeaseCheckpointOutput,
  WorkflowStatusInput,
  WorkflowStatusOutput,
  ErrCodes,
  StartWorkflowInput,
  StartWorkflowOutput,
  GenerateIdempotencyKeyInput,
  GenerateIdempotencyKeyOutput,
} from '@idempotent-transformer/core';
import { WorkflowServiceImplClient } from '../gen/workflow_service_grpc_pb';
import {
  CheckPointRequest,
  CompleteWorkflowRequest,
  GenerateIdempotencyKeyRequest,
  LeaseCheckpointRequest,
  ReleaseCheckpointRequest,
  WorkflowStartRequest,
  WorkflowStatusRequest,
} from '../gen/workflow_service_pb';
import {
  ReleaseLeaseCheckpointInput,
  ReleaseLeaseCheckpointOutput,
} from '@idempotent-transformer/core/dist/base/rpc-adapter/interfaces/release-leas-checkpoint.interface';

const exceptions = Object.values(ErrCodes);
const handleError = (err: ServiceError) => {
  const code = err.details;
  const exception = exceptions.find((errCode) => errCode === code);
  if (exception || code) {
    return new Error(code);
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
  async generateIdempotencyKey(
    input: GenerateIdempotencyKeyInput
  ): Promise<GenerateIdempotencyKeyOutput> {
    const request = new GenerateIdempotencyKeyRequest();
    request.setWorkflowId(input.workflowId);
    request.setFencingToken(input.fencingToken);
    request.setPosition(input.position);
    return new Promise((resolve, reject) => {
      this.client.generate_idempotency_key(request, (err, response) => {
        if (err) {
          reject(handleError(err));
          return;
        }
        resolve({
          idempotencyKey: response.getIdempotencyKey(),
        });
      });
    });
  }

  async startWorkflow(input: StartWorkflowInput): Promise<StartWorkflowOutput> {
    const request = new WorkflowStartRequest();
    request.setWorkflowId(input.workflowId);

    if (input.name) {
      request.setContextName(input.name);
    }

    return new Promise((resolve, reject) => {
      this.client.workflow_start(request, (err, response) => {
        if (err) {
          reject(handleError(err));
          return;
        }
        resolve({
          fencingToken: response.getFencingToken(),
        });
      });
    });
  }

  async checkpoint(input: CheckpointInput): Promise<CheckpointOutput> {
    const request = new CheckPointRequest();
    request.setWorkflowId(input.workflowId);
    request.setValue(input.value as string);
    request.setFencingToken(input.fencingToken);
    request.setPosition(input.position);
    request.setIdempotencyKey(input.idempotencyKey);
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
    request.setWorkflowId(input.workflowId);
    request.setFencingToken(input.fencingToken);
    request.setLeaseTimeout(input.leaseTimeout);
    request.setPosition(input.position);
    request.setIdempotencyKey(input.idempotencyKey);
    return new Promise((resolve, reject) => {
      this.client.lease_checkpoint(request, (err, response) => {
        if (err) {
          reject(handleError(err));
          return;
        }
        resolve({
          value: response.getValue() as Buffer,
          remainingLeaseTimeout: response.getRemainingLeaseTimeout(),
        });
      });
    });
  }

  async completeWorkflow(input: CompleteWorkflowInput): Promise<CompleteWorkflowOutput> {
    const request = new CompleteWorkflowRequest();
    request.setWorkflowId(input.workflowId);
    request.setFencingToken(input.fencingToken);
    request.setExpireAfter(input.expireAfter);
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

  async getWorkflowStatus(input: WorkflowStatusInput): Promise<WorkflowStatusOutput> {
    const request = new WorkflowStatusRequest();
    request.setWorkflowId(input.workflowId);
    return new Promise((resolve, reject) => {
      this.client.workflow_status(request, (err, response) => {
        if (err) {
          reject(handleError(err));
          return;
        }
        resolve({
          id: response.getWorkflowId(),
          status: response.getStatus(),
          expireAt: response.getExpireAt(),
          completedAt: response.getCompletedAt(),
        });
      });
    });
  }

  async releaseLeaseCheckpoint(
    input: ReleaseLeaseCheckpointInput
  ): Promise<ReleaseLeaseCheckpointOutput> {
    const request = new ReleaseCheckpointRequest();
    request.setWorkflowId(input.workflowId);
    request.setPosition(input.position);
    return new Promise((resolve, reject) => {
      this.client.release_checkpoint(request, (err, response) => {
        if (err) {
          reject(handleError(err));
          return;
        }
        resolve({});
      });
    });
  }
}
