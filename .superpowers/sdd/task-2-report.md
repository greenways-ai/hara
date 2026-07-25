# Task 2 — Sample scenes: Tron and Synth

## Implemented

- `website/docs/scenes/tron.hal` — a top-down 2D port of the grid Tron simulation.
- `website/docs/scenes/synth.hal` — a small audio/canvas oscilloscope scene.

Both scenes expose `(scenes.<id>/setup)` and `(scenes.<id>/tick dt-ms zoom pan-x pan-y)` and apply the camera transform `(+ pan-x (* logical zoom))` / `(+ pan-y (* logical zoom))` to every rendered coordinate.

## Tron scene details

- 50×50 logical battlefield with coordinates -25..24.
- Four cycles placed randomly in four corner bases, initially moving toward the arena center (axis-aligned).
- Each step claims the entered cell: +1 for empty, +3 for stealing.
- Five objective zones (four corners + center) give an additional immediate bonus (5 or 10 points).
- Collision kills a cycle on arena walls or any occupied trail/head cell.
- One survivor triggers a round reset after `reset-delay` ms.
- Trails are stored as ordered cell coordinates and trimmed to `tail-max` (120) cells.
- Fixed color per cycle index across rounds.

## Synth scene details

- Draws an animated triangle-wave oscilloscope across a 400-unit logical width.
- Every 500 ms picks a random note from a C-major pentatonic scale and calls `(audio/note freq 0.15 0.04)`.
- The host audio handler attempts to resume a suspended context; if no user gesture has unlocked audio, the call stays silent but does not crash the scene.

## Simplifications compared to `game.js`

- Replaced isometric projection with a flat top-down 2D view.
- Replaced complex zone shapes with five equal axis-aligned rectangles.
- Replaced continuous movement and segment geometry with integer grid steps.
- Replaced line-segment collision with a simple occupancy grid built from trail cells and heads.
- Simplified AI to "prefer straight, turn when blocked, occasional random turn" instead of lookahead scoring and flood-fill space estimation.
- Removed boost mechanics, score hold/home-loss over time, and zone ownership transitions.
- Removed painter's-sort depth rendering; trails are drawn per cycle in index order.

## Verification

- `.venv-docs/bin/mkdocs build --strict -f website/mkdocs.yml` completed successfully (exit 0) with only the upstream Material for MkDocs warning.
- `cargo run --manifest-path rust/Cargo.toml --bin hara -- run website/docs/scenes/{tron,synth}.hal` parses both files successfully; the runtime stops with `Cannot require missing generated namespace: host.browser.canvas`, which is expected because the browser host resources are only available in the WASM/browser target.

## Runtime concerns

- Hara L0 does not expose `Math.sin`/`cos`, so `synth.hal` uses a hand-rolled triangle wave for the oscilloscope.
- No `try/catch` is available in L0, but the host audio call is designed to be safe when the `AudioContext` is suspended, so the scene degrades to silent animation rather than crashing.

## Commit

Implementation commit: `c923093`
