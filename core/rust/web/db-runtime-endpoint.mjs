function requireMethod(target, name) {
  const method = target?.[name];
  if (typeof method !== "function") {
    throw new TypeError(`database runtime endpoint requires ${name}()`);
  }
  return method.bind(target);
}

function messageData(event) {
  return event && typeof event === "object" && "data" in event ? event.data : event;
}

export function endpointFromEventTarget(target, {
  send = null,
  close = null,
  start = true
} = {}) {
  const postMessage = send ?? requireMethod(target, "postMessage");
  const addEventListener = requireMethod(target, "addEventListener");
  const removeEventListener = requireMethod(target, "removeEventListener");
  const handlers = new Map();
  let closed = false;

  return Object.freeze({
    send(message) {
      if (closed) throw new Error("database runtime endpoint is closed");
      postMessage(message);
    },

    listen(listener) {
      if (closed) throw new Error("database runtime endpoint is closed");
      if (typeof listener !== "function") throw new TypeError("listener must be a function");
      const previous = handlers.get(listener);
      if (previous) return () => removeEventListener("message", previous);
      const handler = event => listener(messageData(event));
      handlers.set(listener, handler);
      addEventListener("message", handler);
      if (start && typeof target.start === "function") target.start();
      return () => {
        if (!handlers.delete(listener)) return false;
        removeEventListener("message", handler);
        return true;
      };
    },

    close() {
      if (closed) return false;
      closed = true;
      for (const handler of handlers.values()) removeEventListener("message", handler);
      handlers.clear();
      if (typeof close === "function") close();
      else if (typeof target.close === "function") target.close();
      return true;
    }
  });
}

export function endpointFromMessagePort(port, options = {}) {
  return endpointFromEventTarget(port, options);
}

export function endpointFromWorker(worker) {
  return endpointFromEventTarget(worker, {
    close: () => worker.terminate?.(),
    start: false
  });
}

export function endpointFromSharedWorker(sharedWorker) {
  if (!sharedWorker?.port) throw new TypeError("SharedWorker endpoint requires .port");
  return endpointFromMessagePort(sharedWorker.port);
}

export function endpointFromNodePort(port, { close = false } = {}) {
  const postMessage = requireMethod(port, "postMessage");
  const on = requireMethod(port, "on");
  const off = typeof port.off === "function"
    ? port.off.bind(port)
    : requireMethod(port, "removeListener");
  const handlers = new Map();
  let closed = false;

  return Object.freeze({
    send(message) {
      if (closed) throw new Error("database runtime endpoint is closed");
      postMessage(message);
    },

    listen(listener) {
      if (closed) throw new Error("database runtime endpoint is closed");
      if (typeof listener !== "function") throw new TypeError("listener must be a function");
      if (handlers.has(listener)) return () => off("message", handlers.get(listener));
      const handler = message => listener(message);
      handlers.set(listener, handler);
      on("message", handler);
      return () => {
        if (!handlers.delete(listener)) return false;
        off("message", handler);
        return true;
      };
    },

    close() {
      if (closed) return false;
      closed = true;
      for (const handler of handlers.values()) off("message", handler);
      handlers.clear();
      if (close) port.close?.();
      return true;
    }
  });
}

export function endpointFromParentPort(parentPort) {
  if (!parentPort) throw new TypeError("Node worker runtime requires parentPort");
  return endpointFromNodePort(parentPort);
}
