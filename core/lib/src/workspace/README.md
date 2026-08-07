# Hara Workspace

`workspace.*` is the portable, headless Workspace contract for Hara.

HAL owns serializable descriptors, state transitions, semantic events, view
projections, extension routing and host-effect descriptions. Browser and native
hosts own workers, storage, DOM, canvas, audio and other non-serializable
resources.

The first slice provides:

- `workspace.component/1` component descriptors;
- area and component lookup;
- deterministic initial selection;
- headless area-selection transitions;
- extension-event routing as explicit effects;
- rejected-event effects that preserve state.

Hodos may use component IDs such as `hodos.dev/preview`, but Hara owns the
component descriptor contract. Greenways Studio remains a product composition
above Hodos and Hara Workspace.
