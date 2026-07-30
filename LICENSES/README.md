# License inventory

Unless a file or directory carries a more specific notice, Hara-owned source
code in this repository is licensed under [EPL-2.0](../LICENSE).

This repository also contains material that is not relicensed by the root
license:

| Location | License or status | Handling |
| --- | --- | --- |
| `docs/vendor/hara-ui/`, `specs/vendor/hara-ui/` | MIT | Synchronized snapshots from `hara-lang/hara-ui`; keep their included `LICENSE` files. |
| Git submodules (`archive/`, `docs/`, `extensions/`, `specs/`, and `website/vendor/hara-ui/`) | Independent repositories | Consult the license in that repository; the root license does not change it. |
| `website/sources/ants.hal` | Copyright Rich Hickey; all rights reserved | Do not treat as EPL-2.0. Confirm permission or replace before redistribution. |
| `website/sources/{universe-within,sunlit-landscape,plasma-storm}.hal` | CC BY-NC-SA 3.0 | Keep the existing notice; do not include in an EPL-2.0 distribution. |

Add every new exception here and preserve its original notices. Third-party
dependency notices belong in release artifacts where the dependency license
requires them; this file is not a substitute for a dependency SBOM or notice
bundle.
