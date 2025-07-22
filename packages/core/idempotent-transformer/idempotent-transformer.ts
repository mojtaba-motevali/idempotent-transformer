import { IdempotentCheckSumGenerator, IdempotentLogger, IdempotentSerializer } from '../base';
import { ErrCodes, IdempotentRpcAdapter, WorkflowStatusInput } from '../base/rpc-adapter';

import { WorkflowAbortedException } from './exception/workflow-aborted.exception';
import {
  IdempotentTransformerInput,
  IdempotentTransformerOptions,
  IIdempotentTaskOptions,
  IdempotentRunnerResult,
} from './interfaces';

function isPromise<T = any>(value: any): value is Promise<T> {
  return value !== null && typeof value === 'object' && typeof value.then === 'function';
}

export class IdempotentTransformer {
  private serializer: IdempotentSerializer;
  private logger?: IdempotentLogger;
  private checksumGenerator: IdempotentCheckSumGenerator;
  private rpcAdapter: IdempotentRpcAdapter;
  constructor(input: IdempotentTransformerInput) {
    const { serializer, log, checksumGenerator, rpcAdapter } = input;
    this.serializer = serializer;
    this.logger = log;
    this.checksumGenerator = checksumGenerator;
    this.rpcAdapter = rpcAdapter;
  }

  async startWorkflow(
    workflowId: string,
    { completedRetentionTime, workflowName }: IdempotentTransformerOptions
  ): Promise<IdempotentRunnerResult> {
    // 1 day
    const { fencingToken } = await this.rpcAdapter.startWorkflow({
      workflowId,
      name: workflowName,
    });
    this.logger?.debug(
      `Leased workflow ${workflowId} with fencing token ${fencingToken}  context bound checkpoints`
    );
    let counter = 0;
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
        const tag = `workflowId:${workflowId} | context:${workflowName} | fencingToken:${fencingToken}`;
        this.logger?.debug(`${tag} Executing function`);
        const plainChecksum = `${workflowId}-${counter}`;
        this.logger?.debug(`${tag} checkpointChecksum plain: ${plainChecksum}`);
        const [positionChecksum, idempotencyChecksum] = await Promise.all([
          this.checksumGenerator.generate(plainChecksum),
          this.checksumGenerator.generate(idempotencyKey),
        ]);
        this.logger?.debug(`${tag} checkpointChecksum plain: ${positionChecksum}`);

        while (true) {
          try {
            this.logger?.debug(
              `Leasing checkpoint fencing:${fencingToken}| workflowId:${workflowId}| context:${workflowName}| idempotencyKey:${idempotencyKey}| positionChecksum:${positionChecksum}`
            );
            const checkpoint = await this.rpcAdapter.leaseCheckpoint({
              workflowId,
              fencingToken,
              leaseTimeout: taskOptions?.leaseTimeout || 1000 * 30,
              positionChecksum,
              idempotencyChecksum,
            });
            if (checkpoint.value) {
              const deserializedValue = await this.serializer.deserialize(checkpoint.value);
              counter++;
              return deserializedValue;
            }
            break;
          } catch (err) {
            this.logger?.debug(
              `Error leasing checkpoint for ${tag}: ${err instanceof Error ? err.message : String(err)}`
            );
            if (err instanceof Error && err.message === ErrCodes.CheckpointLeasedByOtherWorker) {
              await this.waitFor(500);
            } else {
              throw err;
            }
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
            positionChecksum,
          });
          throw err;
        }

        const serializedResult = await this.serializer.serialize(taskResult);
        this.logger?.debug(`Serialized result for ${tag} is ${serializedResult}`);
        let abort = false;
        let i = 0;
        while (i < 3) {
          try {
            const checkpointResult = await this.rpcAdapter.checkpoint({
              workflowId,
              fencingToken,
              positionChecksum,
              value: serializedResult,
              idempotencyChecksum,
            });
            abort = checkpointResult.abort;
            break;
          } catch (err) {
            this.logger?.debug(
              `Error checkpointing for ${tag}: ${err instanceof Error ? err.message : String(err)}`
            );
            await this.waitFor(500);
          }
          i++;
        }

        if (abort) {
          throw new WorkflowAbortedException('Workflow aborted by other worker.');
        }

        counter++;
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
