/** Small deterministic clock surface for generated programs that need a host
 * notion of time without gaining access to the unrestricted global object. */
export function createClockCapability({ now = () => performance.now(), sleep = null } = {}) {
  const pause = sleep ?? ((milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)));
  return Object.freeze({
    now: () => Math.max(0, Math.trunc(now())),
    sleep: (milliseconds = 0) => pause(Math.max(0, Number(milliseconds) || 0))
  });
}
