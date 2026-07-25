/**
 * Shared default bootstrap template for studio kernels: ensures the space
 * exists (creating a blank one if missing) and leaves the kernel ready to
 * eval. UI and broker callers share this one template so the bootstrap
 * shape lives in exactly one place.
 */
export function defaultBootstrap(spaceName) {
  return `(do (require [studio.boot :as boot]) (boot/boot! ${JSON.stringify(spaceName)}))`;
}
