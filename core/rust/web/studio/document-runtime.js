export async function activateStudioDocument({
  broker,
  kernel,
  documentId,
  source,
  nodeId,
  canvasRuntime = null,
  canvasId = null,
  requireFirstFrame = false,
  onTaskError = () => {}
}) {
  const candidate = await broker.prepareDocument(kernel, documentId, source, { nodeId });
  let firstFrame = null;
  let taskFailure = null;
  try {
    if (requireFirstFrame) {
      if (!canvasRuntime || !canvasId) throw new Error("DOCUMENT_CANVAS_UNAVAILABLE");
      canvasRuntime.stage(nodeId, canvasId);
      firstFrame = canvasRuntime.waitForFirstRender(nodeId, canvasId);
    }
    if (typeof candidate.value === "string" && candidate.value.startsWith("task-")) {
      const taskRun = broker.evalPreparedDocument(
        candidate,
        `(studio.node/run-task ${JSON.stringify(candidate.value)})`
      );
      if (firstFrame) {
        taskFailure = taskRun.then(
          () => { throw new Error("DOCUMENT_TASK_ENDED_BEFORE_ACTIVATION"); },
          (error) => { onTaskError(error); throw error; }
        );
      } else {
        taskRun.catch(onTaskError);
      }
    }
    if (firstFrame) await (taskFailure ? Promise.race([firstFrame, taskFailure]) : firstFrame);
    const result = broker.commitDocument(candidate);
    if (firstFrame) canvasRuntime.commit(nodeId, canvasId);
    return result;
  } catch (error) {
    broker.discardDocument(candidate);
    if (firstFrame) canvasRuntime.discard(nodeId, canvasId);
    throw error;
  }
}
