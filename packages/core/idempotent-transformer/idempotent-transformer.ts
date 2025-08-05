import { IdempotentLogger, IdempotentSerializer } from '../base';
import { ErrCodes, IdempotentRpcAdapter, WorkflowStatusInput } from '../base/rpc-adapter';

import { WorkflowAbortedException } from './exception/workflow-aborted.exception';
import {
  IdempotentTransformerInput,
  IdempotentTransformerOptions,
  IdempotentRunnerResult,
} from './interfaces';

function isPromise<T = any>(value: any): value is Promise<T> {
  return value !== null && typeof value === 'object' && typeof value.then === 'function';
}

export class IdempotentTransformer {
  private serializer: IdempotentSerializer;
  private logger?: IdempotentLogger;
  private rpcAdapter: IdempotentRpcAdapter;
  constructor(input: IdempotentTransformerInput) {
    const { serializer, log, rpcAdapter } = input;
    this.serializer = serializer;
    this.logger = log;
    this.rpcAdapter = rpcAdapter;
  }

  async startWorkflow(
    workflowId: string,
    { completedRetentionTime, name }: IdempotentTransformerOptions
  ): Promise<IdempotentRunnerResult> {
    // 1 day
    const { fencingToken } = await this.rpcAdapter.startWorkflow({
      workflowId,
      name,
    });
    this.logger?.debug(
      `Leased workflow ${workflowId} with fencing token ${fencingToken}  context bound checkpoints`
    );
    let position = 0;
    const result = {
      complete: async () => {
        this.logger?.debug(`Completing workflow ${workflowId}`);
        await this.rpcAdapter.completeWorkflow({
          workflowId,
          fencingToken,
          expireAfter: completedRetentionTime || 1000 * 60 * 60 * 24,
        });
      },
      execute: async (idempotencyKey, task, taskOptions) => {
        const leaseTimeout = taskOptions?.leaseTimeout || 1000 * 30;
        const tag = `workflowId:${workflowId} | context:${name} | fencingToken:${fencingToken} | position: ${position}`;
        this.logger?.debug(`${tag} Executing function`);
        let errorCount = 0;
        let beforeExecution;

        while (true) {
          try {
            this.logger?.debug(
              `Leasing checkpoint fencing:${fencingToken}| workflowId:${workflowId}| context:${name}| idempotencyKey:${idempotencyKey}| position:${position}`
            );
            const checkpoint = await this.rpcAdapter.leaseCheckpoint({
              workflowId,
              fencingToken,
              leaseTimeout,
              position: position,
              idempotencyKey,
            });
            if (checkpoint.value) {
              const deserializedValue = await this.serializer.deserialize(checkpoint.value);
              position++;
              return deserializedValue;
            } else if (checkpoint.remainingLeaseTimeout) {
              await this.waitFor(checkpoint.remainingLeaseTimeout / 10);
              continue;
            }
            beforeExecution = Date.now();
            break;
          } catch (err) {
            this.logger?.debug(
              `Error leasing checkpoint for ${tag}: ${err instanceof Error ? err.message : String(err)}`
            );
            // The workflow might not have been replicated yet, so try one more time.
            if (
              err instanceof Error &&
              err.message === ErrCodes.FencingTokenNotFound &&
              position === 0 &&
              errorCount < 1
            ) {
              ++errorCount;
              await this.waitFor(100);
              continue;
            }
            throw err;
          }
        }

        this.logger?.debug(
          `Executing function ${tag} with task options ${JSON.stringify(taskOptions)}`
        );
        const intermediateResult = task();
        let taskResult;
        try {
          if (isPromise(intermediateResult)) {
            taskResult = await intermediateResult;
          } else {
            taskResult = intermediateResult;
          }
        } catch (err) {
          // release checkpoint
          await this.rpcAdapter.releaseLeaseCheckpoint({
            workflowId,
            position,
          });
          throw err;
        }

        const serializedResult = await this.serializer.serialize(taskResult);
        this.logger?.debug(`Serialized result for ${tag} is ${serializedResult}`);
        let abort: boolean | undefined;
        const leaseExpiry = beforeExecution + leaseTimeout;
        // Try to checkpoint until the lease expires.
        while (Date.now() < leaseExpiry) {
          try {
            const checkpointResult = await this.rpcAdapter.checkpoint({
              workflowId,
              fencingToken,
              position,
              value: serializedResult,
              idempotencyKey,
            });
            abort = checkpointResult.abort;
            break;
          } catch (err) {
            this.logger?.debug(
              `Error check pointing for ${tag}: ${err instanceof Error ? err.message : String(err)}`
            );
          }
        }
        // if checkpoint was successful, but another worker also started this workflow, then...
        if (abort) {
          throw new WorkflowAbortedException(
            'Workflow task persisted but aborted by other worker.'
          );
        }

        if (typeof abort === 'undefined' && taskOptions?.onAbort) {
          let i = 0;
          const retryCount = taskOptions.onAbort.retryCount || 3;
          const retryDelay = (
            typeof taskOptions.onAbort.retryDelay === 'number'
              ? taskOptions.onAbort.retryDelay
              : 1000
          ) as number;
          while (i < retryCount) {
            try {
              const abortionTask = taskOptions.onAbort.function(taskResult);
              if (isPromise(abortionTask)) {
                await abortionTask;
              }
              break;
            } catch (err) {
              await this.waitFor(retryDelay);
            }
            ++i;
          }
          throw new WorkflowAbortedException('Workflow task rolled back and aborted.');
        }

        position++;
        return taskResult;
      },
      getWorkflowStatus: async (args: WorkflowStatusInput) => {
        return this.rpcAdapter.getWorkflowStatus(args);
      },
    } as IdempotentRunnerResult;

    return result;
  }

  async waitFor(milliseconds: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, milliseconds));
  }
}
