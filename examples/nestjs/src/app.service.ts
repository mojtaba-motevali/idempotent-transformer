import { IdempotentTransformer } from '@idempotent-transformer/core';
import { Inject, Injectable } from '@nestjs/common';
import { appendFile, readFile } from 'fs';
import { ExampleService } from './example/example.service';
import { IDEMPOTENT_TRANSFORMER } from '@idempotent-transformer/nestjs';

@Injectable()
export class AppService {
  constructor(
    @Inject(IDEMPOTENT_TRANSFORMER)
    private readonly idempotent: IdempotentTransformer,
    private readonly paymentProvider: ExampleService,
  ) {}
  getPath(itemId: string): string {
    return `public/${itemId}.json`;
  }

  async savePayment(itemId: string, data: string): Promise<boolean> {
    return new Promise((resolve, reject) => {
      appendFile(this.getPath(itemId), data, (err) => {
        if (err) {
          reject(err);
        } else {
          resolve(true);
        }
      });
    });
  }

  async readPayment(itemId: string): Promise<string> {
    return new Promise((resolve, reject) => {
      readFile(this.getPath(itemId), (err, data) => {
        if (err) {
          reject(err);
          return;
        }
        resolve(data.toString());
      });
    });
  }

  async handlePayment(
    actionId: string,
    data: { itemId: string; amount: number },
  ): Promise<string> {
    const runner = await this.idempotent.startWorkflow(actionId, {
      completedRetentionTime: 60000,
      name: 'Example Existing Workflow',
    });
    const idempotencyKey: string = await runner.generateIdempotencyKey();
    const referenceId = await runner.execute(idempotencyKey, () =>
      this.paymentProvider.createPayment(idempotencyKey, data),
    );
    const json = JSON.stringify({
      ...data,
      referenceId,
    });
    await runner.execute(`Save payment`, () =>
      this.savePayment(data.itemId, json),
    );

    const result = await runner.execute('Get payment', () =>
      this.readPayment(data.itemId),
    );

    await runner.complete();
    return result;
  }
}
