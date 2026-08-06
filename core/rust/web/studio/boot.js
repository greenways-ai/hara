/**
 * Shared language bootstrap. The host creates and attaches the filesystem
 * mount before evaluating this template; HAL only receives project identity.
 */
export function defaultBootstrap(spaceName) {
  return `(do (require [studio.boot :as boot]) (boot/boot! ${JSON.stringify(spaceName)}))`;
}
