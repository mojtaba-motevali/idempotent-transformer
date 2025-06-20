'use client';

import {
  IdempotentCrypto,
  IdempotentSerializer,
  IdempotentStateStore,
  TSerialized,
} from '@idempotent-transformer/core';
import crypto from 'crypto';

export class Md5Crypto extends IdempotentCrypto {
  constructor() {
    super();
  }
  async createHash(data: TSerialized): Promise<string> {
    return crypto.createHash('md5').update(data).digest('hex');
  }
}

export class JsonSerializer extends IdempotentSerializer {
  constructor() {
    super();
  }

  async serialize<T>(data: T): Promise<TSerialized> {
    return JSON.stringify(data);
  }
  async deserialize<T>(data: TSerialized): Promise<T> {
    return JSON.parse(data as string);
  }
  async configure(): Promise<void> {
    return;
  }
}
export class Storage extends IdempotentStateStore {
  constructor() {
    super();
  }
  async connect(): Promise<void> {}
  async disconnect(): Promise<void> {}
  async isConnected(): Promise<boolean> {
    return true;
  }
  async find(key: string): Promise<string | null> {
    return localStorage.getItem(key);
  }
  async save(key: string, value: string): Promise<void> {
    localStorage.setItem(key, value);
  }
}
