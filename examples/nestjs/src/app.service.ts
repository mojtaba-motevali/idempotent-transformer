import { IdempotentTransformer } from '@idempotent-transformer/core';
import { IDEMPOTENT_TRANSFORMER } from '@idempotent-transformer/nestjs';
import { Inject, Injectable } from '@nestjs/common';
import { appendFile, readFile } from 'fs';
import { ExampleService } from './example/example.service';

@Injectable()
export class AppService {
  constructor(
    @Inject(IDEMPOTENT_TRANSFORMER)
    private readonly idempotent: IdempotentTransformer,
    private readonly exampleService: ExampleService,
  ) {}

  async writeFile(path: string, data: string): Promise<boolean> {
    return new Promise((resolve, reject) => {
      appendFile(path, data, (err) => {
        if (err) {
          reject(err);
        } else {
          resolve(true);
        }
      });
    });
  }

  async readFile(path: string): Promise<string> {
    return new Promise((resolve, reject) => {
      readFile(path, (err, data) => {
        if (err) {
          reject(err);
        }
        resolve(data.toString());
      });
    });
  }

  async writeJSON(actionId: string, data: { name: string }): Promise<string> {
    const idempotent = await this.idempotent.makeIdempotent(actionId, {
      writeFile: (...args: Parameters<typeof this.writeFile>) =>
        this.writeFile(...args),
      ping: (...args: Parameters<typeof this.exampleService.ping>) =>
        this.exampleService.ping(...args),
    });
    const path = `public/${data.name}.json`;
    const ping = await idempotent.ping({
      shouldCompress: true,
    });
    const json = JSON.stringify({
      ...data,
      randomId: ping,
    });

    await idempotent.writeFile(path, json, {
      shouldCompress: true,
    });
    return await this.readFile(path);
  }
}
