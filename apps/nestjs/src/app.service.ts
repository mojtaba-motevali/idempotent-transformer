import { IdempotentTransformer } from '@idempotent-transformer/core';
import { IDEMPOTENT_TRANSFORMER } from '@idempotent-transformer/nestjs';
import { Inject, Injectable } from '@nestjs/common';
import { appendFile, readFile } from 'fs';

@Injectable()
export class AppService {
  constructor(
    @Inject(IDEMPOTENT_TRANSFORMER)
    private readonly idempotent: IdempotentTransformer,
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

  async writeJSON(actionId: string, data: any): Promise<string> {
    const idempotent = this.idempotent.makeIdempotent(actionId, {
      writeFile: this.writeFile,
    });
    const path = `${data.name}.json`;
    const json = JSON.stringify(data);
    await idempotent.writeFile(path, json, {
      shouldCompress: true,
    });
    return await this.readFile(path);
  }
}
