export interface IIdempotentTaskOptions<T> {
  /**
   * The number of milliseconds when the task result will be considered expired.
   * @default 30 seconds
   */
  leaseTimeout?: number;

  /**
   * The function to call when the task is completed but failed to be persisted due to other worker executing this task due to lease timeout expiry
   * or connectivity issue.
   */
  onAbort?: {
    function: (result: T) => Promise<void> | void;
    /**
     * The number of times to retry the function in case of failure.
     * @default 3
     */
    retryCount?: number;
    /**
     * The number of milliseconds to wait before retrying the function in case of failure.
     * @default 1000
     */
    retryDelay?: number;
  };
}
