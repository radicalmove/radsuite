import { describe, expect, test } from "vitest";
import { createAsyncRequestGuard } from "./asyncRequestGuard";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((nextResolve) => {
    resolve = nextResolve;
  });
  return { promise, resolve };
}

describe("async request guard", () => {
  test("prevents a deferred update check from applying after component teardown", async () => {
    const guard = createAsyncRequestGuard();
    const request = deferred<void>();
    const isCurrent = guard.begin();
    let updateMessage: string | null = null;
    let timersScheduled = 0;

    const pendingCheck = request.promise.then(() => {
      if (!isCurrent()) return;
      updateMessage = "RADsuite v0.2.7 is up to date.";
      timersScheduled += 1;
    });

    guard.dispose();
    request.resolve();
    await pendingCheck;

    expect(updateMessage).toBeNull();
    expect(timersScheduled).toBe(0);
  });
});
