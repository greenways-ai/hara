import assert from "node:assert/strict";
import test from "node:test";
import { EventEmitter } from "node:events";
import {
  endpointFromMessagePort,
  endpointFromNodePort,
  endpointFromSharedWorker,
  endpointFromWorker
} from "./db-runtime-endpoint.mjs";

class LinkedEventPort extends EventTarget {
  peer = null;
  started = false;
  closed = false;

  postMessage(message) {
    this.peer?.dispatchEvent(new MessageEvent("message", { data: message }));
  }

  start() {
    this.started = true;
  }

  close() {
    this.closed = true;
  }
}

function linkedEventPorts() {
  const left = new LinkedEventPort();
  const right = new LinkedEventPort();
  left.peer = right;
  right.peer = left;
  return [left, right];
}

class LinkedNodePort extends EventEmitter {
  peer = null;
  closed = false;

  postMessage(message) {
    this.peer?.emit("message", message);
  }

  close() {
    this.closed = true;
  }
}

function linkedNodePorts() {
  const left = new LinkedNodePort();
  const right = new LinkedNodePort();
  left.peer = right;
  right.peer = left;
  return [left, right];
}

test("MessagePort endpoints send, listen, start and close", () => {
  const [leftPort, rightPort] = linkedEventPorts();
  const left = endpointFromMessagePort(leftPort);
  const right = endpointFromMessagePort(rightPort);
  const messages = [];
  right.listen(message => messages.push(message));
  left.send({ kind: "request", id: "req-1" });
  assert.deepEqual(messages, [{ kind: "request", id: "req-1" }]);
  assert.equal(rightPort.started, true);
  assert.equal(right.close(), true);
  assert.equal(rightPort.closed, true);
  assert.throws(() => right.send({}), /closed/);
});

test("Worker endpoints terminate rather than calling close", () => {
  const [worker, peer] = linkedEventPorts();
  worker.terminated = false;
  worker.terminate = () => {
    worker.terminated = true;
  };
  const endpoint = endpointFromWorker(worker);
  const remote = endpointFromMessagePort(peer);
  const messages = [];
  remote.listen(message => messages.push(message));
  endpoint.send({ signal: "ready" });
  assert.deepEqual(messages, [{ signal: "ready" }]);
  endpoint.close();
  assert.equal(worker.terminated, true);
});

test("SharedWorker endpoints use and start the worker port", () => {
  const [port, peer] = linkedEventPorts();
  const endpoint = endpointFromSharedWorker({ port });
  const remote = endpointFromMessagePort(peer);
  const messages = [];
  endpoint.listen(message => messages.push(message));
  remote.send({ kind: "response", id: "res-1" });
  assert.deepEqual(messages, [{ kind: "response", id: "res-1" }]);
  assert.equal(port.started, true);
});

test("Node parentPort endpoints use EventEmitter message semantics", () => {
  const [leftPort, rightPort] = linkedNodePorts();
  const left = endpointFromNodePort(leftPort, { close: true });
  const right = endpointFromNodePort(rightPort, { close: true });
  const messages = [];
  right.listen(message => messages.push(message));
  left.send({ action: "@xt.db/query", args: [] });
  assert.deepEqual(messages, [{ action: "@xt.db/query", args: [] }]);
  assert.equal(left.close(), true);
  assert.equal(leftPort.closed, true);
  assert.throws(() => left.send({}), /closed/);
});

test("duplicate listeners are installed once and can be removed", () => {
  const [leftPort, rightPort] = linkedEventPorts();
  const left = endpointFromMessagePort(leftPort);
  const right = endpointFromMessagePort(rightPort);
  let count = 0;
  const listener = () => { count += 1; };
  const remove = right.listen(listener);
  right.listen(listener);
  left.send(1);
  assert.equal(count, 1);
  assert.equal(remove(), true);
  left.send(2);
  assert.equal(count, 1);
});
