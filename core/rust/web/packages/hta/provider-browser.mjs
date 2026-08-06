import { decodeHta, encodeHta } from "./index.js";
import { providerError, toHta } from "./provider-common.mjs";

export function serveBrowserProvider(call, options = {}) {
  const scope = options.scope ?? self;
  const cancelled = new Set();

  scope.addEventListener("message", async event => {
    const message = event.data;
    try {
      if (message.type === "init") {
        scope.postMessage({ type: "ready" });
      } else if (message.type === "cancel") {
        cancelled.add(message.id);
      } else if (message.type === "close") {
        scope.close();
      } else if (message.type === "call") {
        const [operation, args] = decodeHta(message.frame);
        try {
          const value = await call(operation, args);
          if (!cancelled.delete(message.id)) {
            scope.postMessage({
              type: "result",
              id: message.id,
              ok: true,
              frame: encodeHta(toHta(value))
            });
          }
        } catch (error) {
          if (!cancelled.delete(message.id)) {
            scope.postMessage({
              type: "result",
              id: message.id,
              ok: false,
              frame: encodeHta(providerError(error, "browser", options.errorCode))
            });
          }
        }
      }
    } catch (error) {
      scope.postMessage({ type: "fatal", error: { message: String(error?.message ?? error) } });
    }
  });
}
