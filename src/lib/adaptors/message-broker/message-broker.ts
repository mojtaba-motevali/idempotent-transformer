export interface MessageBrokerInput {
  log: (message: string, level: 'info' | 'error' | 'warn' | 'debug') => void;
}

export class MessageBroker {
  constructor(input: MessageBrokerInput) {}
}
