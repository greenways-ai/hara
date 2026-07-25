export const DEFAULT_BEHAVIOUR =
  "(if (> x 760) -3 (if (< x 40) 3 velocity))";

export function createWorld() {
  return {
    tick: 0,
    updatedAt: performance.now(),
    agent: { id: "fox-12", x: 120, y: 220, velocity: 3 },
    behaviour: DEFAULT_BEHAVIOUR
  };
}

export function stepWorld(world, evaluate) {
  const velocity = evaluate(world.behaviour, {
    x: Math.round(world.agent.x),
    velocity: Math.round(world.agent.velocity)
  });
  const nextVelocity = Number.isFinite(velocity)
    ? Math.max(-12, Math.min(12, velocity))
    : world.agent.velocity;
  return {
    ...world,
    tick: world.tick + 1,
    updatedAt: performance.now(),
    agent: {
      ...world.agent,
      x: world.agent.x + nextVelocity,
      velocity: nextVelocity
    }
  };
}

export function validWorld(value) {
  return Boolean(
    value &&
    Number.isSafeInteger(value.tick) &&
    value.agent?.id === "fox-12" &&
    Number.isFinite(value.agent.x) &&
    Number.isFinite(value.agent.y) &&
    Number.isFinite(value.agent.velocity) &&
    typeof value.behaviour === "string"
  );
}

export function evaluationSource(source, bindings) {
  if (typeof source !== "string" || !source.trim() || source.length > 4096) {
    throw new Error("Behaviour must contain between 1 and 4096 characters");
  }
  return `(let (x ${bindings.x} velocity ${bindings.velocity}) ${source})`;
}
