# Development and production flow

Hara uses two long-lived branches:

- `main` is the integration branch. Feature work reaches it through pull requests.
- `prod` is the production branch. Promotion from `main` to `prod` deploys public services.

The normal flow is:

1. Open a pull request from a feature branch to `main`.
2. `Core CI` runs once, while relevant HAL, Tahto, collection, and `std.db` checks run only when their paths change.
3. Merge the validated change into `main`.
4. Open a promotion pull request from `main` to `prod`.
5. `Core CI` validates the production candidate.
6. Merge into `prod`; path-scoped website and registry workflows test and deploy the affected services.

The full runtime conformance matrix is intentionally manual. Run it before releases or when runtime, compiler, VM, WASM, Truffle, native-image, or benchmark behavior changes.

Package publication remains tag-driven or manually dispatched. Publication is separate from branch promotion because registry releases are immutable.
