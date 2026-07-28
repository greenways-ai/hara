/** Restricts a generated node to the canvas it explicitly claims. The
 * CanvasRuntime remains responsible for ownership, frame scheduling, input,
 * WebGL resource disposal, and generation replacement. */
export function createCanvasCapability(canvasRuntime) {
  if (!canvasRuntime) throw new Error("createCanvasCapability requires CanvasRuntime");
  return Object.freeze({
    forNode({ nodeId }) {
      return Object.freeze({
        claim: (canvasId) => canvasRuntime.claim(nodeId, canvasId),
        stage: (canvasId) => canvasRuntime.stage(nodeId, canvasId),
        commit: (canvasId) => canvasRuntime.commit(nodeId, canvasId),
        discard: (canvasId) => canvasRuntime.discard(nodeId, canvasId),
        release: (canvasId = null) => canvasRuntime.release(nodeId, canvasId),
        nextFrame: (canvasId) => canvasRuntime.nextFrame(nodeId, canvasId),
        render: (canvasId, frame) => canvasRuntime.render(nodeId, canvasId, frame),
        waitForFirstRender: (canvasId, timeout) => canvasRuntime.waitForFirstRender(nodeId, canvasId, timeout)
      });
    }
  });
}
