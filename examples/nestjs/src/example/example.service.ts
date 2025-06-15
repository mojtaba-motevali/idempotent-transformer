import { Injectable } from '@nestjs/common';

@Injectable()
export class ExampleService {
  constructor() {}

  async ping(): Promise<string> {
    return new Promise((resolve) => {
      setTimeout(() => {
        resolve(Math.random().toString(36).substring(2, 15));
      }, 1000);
    });
  }
}
