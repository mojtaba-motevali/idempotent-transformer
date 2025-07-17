export interface IIdempotentTaskOptions {
  /**
   * The number of milliseconds when the task result will be considered expired.
   * @default 30 seconds
   */
  leaseTimeout?: number;
}
