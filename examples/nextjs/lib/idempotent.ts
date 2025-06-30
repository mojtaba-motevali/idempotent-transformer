'use client';

import {
  IdempotentCrypto,
  IdempotentSerializer,
  IdempotentStateStore,
  TSerialized,
  SERIALIZE_NAME_METADATA_KEY,
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

  private replacer = (_key: string, value: any): any => {
    if (value && typeof value === 'object') {
      const proto = Object.getPrototypeOf(value);
      const key = proto?.constructor?.[SERIALIZE_NAME_METADATA_KEY];
      if (key && IdempotentSerializer.decoratedModels.has(key)) {
        const { serializeMethodName } = IdempotentSerializer.decoratedModels.get(key)!;
        return {
          __type: key,
          value: value[serializeMethodName](),
        };
      }
    }
    return value;
  };

  private reviver = (_key: string, value: any): any => {
    if (
      value &&
      typeof value === 'object' &&
      value.__type &&
      IdempotentSerializer.decoratedModels.has(value.__type)
    ) {
      const { class: ModelClass, deserializeMethodName } = IdempotentSerializer.decoratedModels.get(
        value.__type
      )!;
      return (ModelClass as any)[deserializeMethodName](value.value);
    }
    return value;
  };

  async serialize(data: any): Promise<TSerialized> {
    return JSON.stringify(data, this.replacer);
  }

  async deserialize(data: TSerialized): Promise<any> {
    return JSON.parse(data as string, this.reviver);
  }

  async configure(): Promise<void> {}
}
