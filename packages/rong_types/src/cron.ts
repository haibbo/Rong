export type RongCronHandlerResult = void | PromiseLike<void> | unknown;

/**
 * Function invoked when an in-process cron job fires.
 *
 * `this` is bound to the CronJob handle. Sync handlers and async handlers are
 * both supported. If the handler returns a Promise, Rong waits for it to
 * settle before that tick is considered complete.
 */
export type RongCronHandler = (this: RongCronJob) => RongCronHandlerResult;

export interface RongCronJob {
  /** The normalized five-field cron expression for this in-process job. */
  readonly cron: string;

  /** Stop this in-process job. Chainable. */
  stop(): this;

  /** Keep the in-process job referenced. Chainable. */
  ref(): this;

  /** Allow the in-process job to be unreferenced. Chainable. */
  unref(): this;
}

export interface RongCronFunction {
  /**
   * Register an in-process cron job and synchronously return a CronJob handle.
   *
   * The handler may be sync or async. Overlapping ticks for the same job are
   * skipped while a previous handler invocation is still running.
   */
  (schedule: string, handler: RongCronHandler): RongCronJob;

  /**
   * Return the next UTC Date matched by a five-field cron expression, or null
   * when no future match exists.
   */
  parse(expression: string, relativeDate?: Date | number): Date | null;
}
