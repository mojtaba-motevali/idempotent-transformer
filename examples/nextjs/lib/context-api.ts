import { IdempotentTransformer } from '@idempotent-transformer/core';
import { createContext } from 'react';

export const IdempotentProviderContext = createContext<IdempotentTransformer | null>(null);
