export type AsyncRequestGuard = {
  begin: () => () => boolean;
  dispose: () => void;
};

export function createAsyncRequestGuard(): AsyncRequestGuard {
  let disposed = false;
  let generation = 0;

  return {
    begin() {
      const requestGeneration = ++generation;
      return () => !disposed && requestGeneration === generation;
    },
    dispose() {
      disposed = true;
      generation += 1;
    },
  };
}
