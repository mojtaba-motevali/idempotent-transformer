export interface IdempotentTransformerInput {
  log: (message: string, level: 'info' | 'error' | 'warn' | 'debug') => void;
  broker: {
    publish: (topic: string, message: string) => Promise<void>;
    subscribe: (topic: string, callback: (message: string) => void) => Promise<void>;
  };
  storage: 'postgres';
}

export interface IdempotentTransformerOutput {}
