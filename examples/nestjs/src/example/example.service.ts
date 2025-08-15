import { Injectable } from '@nestjs/common';

@Injectable()
export class ExampleService {
  constructor() {}

  async createPayment(
    idempotencyKey: string,
    data: { itemId: string; amount: number },
  ): Promise<string> {
    console.log('idempotencyKey', idempotencyKey);
    console.log('data', data);
    return new Promise((resolve) => {
      setTimeout(() => {
        resolve(Math.random().toString(36).substring(2, 15));
      }, 1000);
    });
  }
}
