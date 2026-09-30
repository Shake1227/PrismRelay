export function createRequestGuard() {
  let revision = 0;
  return {
    invalidate() {
      revision += 1;
    },
    begin() {
      revision += 1;
      const current = revision;
      return () => current === revision;
    },
  };
}
