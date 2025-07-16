import { IdempotentCheckSumGenerator, IdempotentLogger, IdempotentSerializer } from '../base';
import { IdempotentRpcAdapter } from '../base/rpc-adapter';
import { CheckpointLeasedByOtherWorkerException } from '../base/rpc-adapter/exceptions';
import { WorkflowAbortedException } from './exception/workflow-aborted.exception';
import {
  IdempotentTransformerInput,
  IdempotentTransformerOptions,
  IIdempotentTaskOptions,
  IdempotentRunnerResult,
} from './interfaces';

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
    {
      prefetchCheckpoints = false,
      retentionTime,
      contextName: workflowContextName,
      isNested = false,
    }: IdempotentTransformerOptions
  ): Promise<IdempotentRunnerResult> {
    // 1 day
    const { fencing_token, checkpoints, total_context_bound_checkpoints } =
      await this.rpcAdapter.leaseWorkflow({
        workflow_id: workflowId,
        is_nested: isNested,
        prefetch_checkpoints: prefetchCheckpoints,
        context_name: workflowContextName,
      });
    this.logger?.debug(
      `Leased workflow ${workflowId} with fencing token ${fencing_token} and ${total_context_bound_checkpoints} context bound checkpoints`
    );
    let counter = 0;
    const result = {
      complete: async () => {
        this.logger?.debug(`Completing workflow ${workflowId}`);
        await this.rpcAdapter.completeWorkflow({
          workflow_id: workflowId,
          fencing_token,
          expire_after: retentionTime || 1000 * 60 * 60 * 24,
        });
      },
      execute: async (
        idempotencyKey: string,
        fn: (...args: any[]) => Promise<any>,
        taskOptions?: IIdempotentTaskOptions
      ) => {
        this.logger?.debug(`Executing function ${idempotencyKey}`);
        const plain_checksum = `${workflowId}-${workflowContextName}-${idempotencyKey}-${counter}`;
        this.logger?.debug(`checkpoint_checksum plain: ${plain_checksum}`);
        const checkpoint_checksum = await this.checksumGenerator.generate(plain_checksum);
        this.logger?.debug(`checkpoint_checksum plain: ${checkpoint_checksum}`);

        const totalCheckpoints = Object.keys(checkpoints).length;
        this.logger?.debug(
          `Prefetched ${totalCheckpoints} checkpoints for ${workflowId} with context ${workflowContextName} and idempotency key ${idempotencyKey}`
        );
        // if prefetchCheckpoints is true, then we need to check if the checkpoint is already in the cache
        if (totalCheckpoints > 0) {
          const checkpointValue = checkpoints[checkpoint_checksum.toString()];
          this.logger?.debug(
            `Found checkpoint ${checkpoint_checksum} for ${workflowId} with context ${workflowContextName} and idempotency key ${idempotencyKey}`
          );
          if (checkpointValue) {
            const deserializedValue = await this.serializer.deserialize(checkpointValue);
            counter++;
            return deserializedValue;
          } else {
            throw new Error('Undeterministic checkpoint found');
          }
        }

        this.logger?.debug(
          `No cached checkpoints found for ${workflowId} with context ${workflowContextName} and idempotency key ${idempotencyKey}`
        );
        // if total_context_bound_checkpoints is greater than 0, then we need to lease a checkpoint
        if (total_context_bound_checkpoints > 0) {
          while (true) {
            try {
              this.logger?.debug(
                `Leasing checkpoint for ${workflowId} with context ${workflowContextName} and idempotency key ${idempotencyKey}`
              );
              const checkpoint = await this.rpcAdapter.leaseCheckpoint({
                workflow_id: workflowId,
                fencing_token,
                lease_timeout: taskOptions?.leaseTimeout || 1000 * 30,
                position_checksum: checkpoint_checksum,
              });
              if (checkpoint.value) {
                const deserializedValue = await this.serializer.deserialize(checkpoint.value);
                counter++;
                return deserializedValue;
              }
              break;
            } catch (err) {
              this.logger?.debug(
                `Error leasing checkpoint for ${workflowId} with context ${workflowContextName} and idempotency key ${idempotencyKey}: ${err instanceof Error ? err.message : String(err)}`
              );
              if (err instanceof CheckpointLeasedByOtherWorkerException) {
                await this.waitFor(500);
              } else {
                throw err;
              }
            }
          }
        }

        this.logger?.debug(
          `Executing function ${idempotencyKey} with task options ${JSON.stringify(taskOptions)}`
        );
        const functionExecutionResult = await fn();

        const serializedResult = await this.serializer.serialize(functionExecutionResult);
        this.logger?.debug(
          `Serialized result for ${workflowId} with context ${workflowContextName} and idempotency key ${idempotencyKey} is ${serializedResult}`
        );
        let abort = false;
        let i = 0;
        while (i < 3) {
          try {
            const checkpointResult = await this.rpcAdapter.checkpoint({
              workflow_id: workflowId,
              fencing_token,
              position_checksum: checkpoint_checksum,
              value: serializedResult,
              workflow_context_name: workflowContextName,
              checkpoint_context_name: idempotencyKey,
            });
            abort = checkpointResult.abort;
            break;
          } catch (err) {
            this.logger?.debug(
              `Error checkpointing for ${workflowId} with context ${workflowContextName} and idempotency key ${idempotencyKey}: ${err instanceof Error ? err.message : String(err)}`
            );
            await this.waitFor(500);
          }
          i++;
        }

        if (abort) {
          throw new WorkflowAbortedException('Workflow aborted by other worker.');
        }

        counter++;
        return functionExecutionResult;
      },
    } as IdempotentRunnerResult;

    return result;
  }

  async waitFor(milliseconds: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, milliseconds));
  }
}
