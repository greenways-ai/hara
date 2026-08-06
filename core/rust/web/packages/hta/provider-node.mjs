import { decodeHta, encodeHta } from "./index.js";
import { providerError, toHta } from "./provider-common.mjs";

export function serveNodeProvider(call, options = {}) {
  const input = options.input ?? process.stdin;
  const output = options.output ?? process.stdout;
  const exit = options.exit ?? (code => process.exit(code));
  const maxFrameSize = options.maxFrameSize ?? 64 * 1024 * 1024;
  const cancelled = new Set();
  let buffered = new Uint8Array();
  let expected = null;

  if (options.redirectConsole !== false) {
    console.log = (...values) => console.error(...values);
    console.info = (...values) => console.error(...values);
  }

  input.on("data", chunk => {
    const next = new Uint8Array(buffered.length + chunk.length);
    next.set(buffered);
    next.set(chunk, buffered.length);
    buffered = next;
    drain();
  });
  input.on("end", () => exit(0));

  function drain() {
    while (true) {
      if (expected === null) {
        if (buffered.length < 4) return;
        expected = new DataView(buffered.buffer, buffered.byteOffset, 4).getUint32(0, false);
        buffered = buffered.slice(4);
        if (expected === 0 || expected > maxFrameSize) {
          throw new Error("hta/process-frame-size");
        }
      }
      if (buffered.length < expected) return;
      const frame = buffered.slice(0, expected);
      buffered = buffered.slice(expected);
      expected = null;
      void dispatch(decodeHta(frame));
    }
  }

  async function dispatch(frame) {
    const [kind, id, operation, args] = frame;
    if (kind === "handshake") {
      write(["ready", 1]);
      return;
    }
    if (kind === "shutdown") {
      exit(0);
      return;
    }
    if (kind === "cancel") {
      cancelled.add(Number(id));
      return;
    }
    if (kind !== "call") throw new Error(`hta/process-event-unknown: ${kind}`);
    const requestId = Number(id);
    try {
      const value = await call(operation, args);
      if (!cancelled.delete(requestId)) write(["result", requestId, toHta(value)]);
    } catch (error) {
      if (!cancelled.delete(requestId)) {
        write(["error", requestId, providerError(error, "node", options.errorCode)]);
      }
    }
  }

  function write(value) {
    const frame = encodeHta(value);
    const header = new Uint8Array(4);
    new DataView(header.buffer).setUint32(0, frame.length, false);
    output.write(header);
    output.write(frame);
  }
}
