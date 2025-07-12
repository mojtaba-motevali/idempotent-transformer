import { TSerialized } from '../../base';

export interface IdempotencyResult {
  taskId: string;
  workflowId: string;
  value: TSerialized;
}
