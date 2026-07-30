/**
 * Product-neutral browser embedding for Hara.
 *
 * `hara-web` owns host capability composition and kernel/session lifecycle.
 * Greenways Studio is one consumer of this surface; it should not own the
 * browser runtime contract.
 */
import { createBrowserBroker } from "./host/broker.js";
import { createHostDescription, createHostServices } from "./host/services.js";

export function createHaraWebHost({
  workerUrl,
  sharedWorkerUrl,
  moduleBytes,
  resources = {},
  hostCalls,
  hostOptions = {},
  createBroker = createBrowserBroker,
  onKernelStarting,
  onKernelCreated,
  onKernelClosed
} = {}) {
  const services = hostCalls ?? createHostServices(hostOptions);
  const broker = createBroker({
    workerUrl,
    sharedWorkerUrl,
    moduleBytes,
    resources,
    hostCalls: services,
    onKernelStarting,
    onKernelCreated,
    onKernelClosed
  });
  // The wire handlers return HTA values.  Keep the embedding API plain JS so
  // a product never has to decode its own host descriptor.
  const description = createHostDescription(hostOptions);

  return Object.freeze({
    describe: () => description,
    capabilities: () => description["host/granted"],
    capability: (name) => description["host/granted"].includes(String(name).replace(/^:/, "")),
    kernels: Object.freeze({
      create: (...args) => broker.create(...args),
      require: (...args) => broker.require(...args),
      close: (...args) => broker.close(...args),
      list: () => broker.list(),
      eval: (...args) => broker.eval(...args)
    }),
    sessions: Object.freeze({
      create: (...args) => broker.createSession(...args),
      close: (...args) => broker.closeSession(...args),
      list: (...args) => broker.listSessions(...args),
      eval: (...args) => broker.evalSession(...args)
    })
  });
}

export { createHostDescription, createHostServices };
