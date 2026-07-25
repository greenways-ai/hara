# Kernel

HARA is not only a language — it is a small Lisp kernel that runs in the browser.

## Sessions

Local browser sessions backed by the HARA wasm runtime. Each session carries its own state, namespace, and evaluation history.

## State transitions

Every meaningful change in a session is logged as a transition. These transitions can be replayed, verified, and shared without exposing the full session record.

## Agents

Small autonomous HARA programs that interact through the kernel. Agents can compete, mediate, or collaborate inside a shared session.

## Proofs

A lightweight proof system lets two parties verify that a session transitioned correctly without revealing the code or data that produced the transition.

## Competitions, mediation, and collaboration

The kernel provides primitives for:

- **Competitions** — agents compete in a sandboxed environment (e.g., multiplayer TRON).
- **Mediation** — a neutral party validates transitions between untrusted participants.
- **Collaboration** — multiple identities contribute to a shared session while keeping their local state consistent.
