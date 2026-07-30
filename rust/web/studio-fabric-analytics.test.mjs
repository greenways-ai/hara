import assert from "node:assert/strict";
import test from "node:test";

import {
  FabricAnalyticsController,
  FabricAnalyticsModel
} from "./studio/fabric-analytics.js";

const topology = {
  node: "node-a",
  spaces: [{
    id: "room",
    sessions: [
      { id: "researcher", shard: 0, home: "node-a", namespaces: ["agent.tool"] },
      { id: "reviewer", shard: 1, home: "node-a", namespaces: [] }
    ]
  }]
};

test("fabric analytics builds redacted traffic flows from service events", () => {
  const model = new FabricAnalyticsModel();
  model.ingestTopology(JSON.stringify(topology));
  model.ingestEvents([JSON.stringify({
    cursor: 4,
    timestamp_ms: 100,
    kind: "report/accepted",
    space: "room",
    session: "reviewer",
    detail: {
      source: "researcher", signal: "finding", bytes: "128",
      delivered: "1", retained: "true"
    }
  })]);
  const snapshot = model.snapshot();
  assert.equal(snapshot.spaces[0].sessions[0].namespaces[0], "agent.tool");
  assert.deepEqual(snapshot.flows[0], {
    space: "room", source: "researcher", target: "reviewer", signal: "finding",
    reports: 1, bytes: 128, delivered: 1, retained: 1,
    lastCursor: 4, lastTimestamp: 100
  });
  assert.equal(JSON.stringify(snapshot).includes("payload"), false);
});

test("fabric analytics ignores duplicate cursors and controller resumes polling", async () => {
  const cursors = [];
  const controller = new FabricAnalyticsController({
    adapter: {
      async topology() { return topology; },
      async events(cursor) {
        cursors.push(cursor);
        return [{
          cursor: 1, timestamp_ms: 10, kind: "session/created",
          space: "room", session: "reviewer", detail: {}
        }];
      }
    }
  });
  await controller.refresh();
  await controller.refresh();
  assert.deepEqual(cursors, [0, 1]);
  assert.equal(controller.model.snapshot().events.length, 1);
});
