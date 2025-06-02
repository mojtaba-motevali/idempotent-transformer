export interface IStateStoreOptions {
  /**
   * The number of milliseconds when the task result will be considered expired.
   */
  ttl: number | null;
  /**
   * Context of the task that was executed.
   */
  context: {
    taskName: string;
  };
}

export abstract class StateStore {
  abstract find(id: string): Promise<Uint8Array<ArrayBufferLike> | null>;
  abstract save(
    id: string,
    value: Uint8Array<ArrayBufferLike>,
    options: IStateStoreOptions
  ): Promise<void>;
}
