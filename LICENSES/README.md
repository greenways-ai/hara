# License inventory

Unless a file or directory carries a more specific notice, Hara-owned source
code in this repository is licensed under [Apache-2.0](../LICENSE).

This repository also contains material that is not relicensed by the root
license:

| Location | License or status | Handling |
| --- | --- | --- |
| `hara-specs/vendor/hara-ui/` | MIT | Synchronized snapshot from `hara-lang/hara-ui`; keep its included `LICENSE` file. |
| `application/greenways-os/extension/hara-chrome/ui/hara-ui/` | MIT | Synced snapshot from `hara-lang/hara-ui`; keep the included `LICENSE` file and do not rewrite its manifest metadata. |
| External repositories (`hara-archive`, `hara-specs`, `hara-www`, `greenways-os`) | Independent repositories | Consult the license in each repository; the root license does not change it. |
| `hara-www/sources/ants.hal` | Copyright Rich Hickey; all rights reserved | Do not treat as Apache-2.0. Confirm permission or replace before redistribution. |
| `hara-www/sources/{universe-within,sunlit-landscape,plasma-storm}.hal` | CC BY-NC-SA 3.0 | Keep the existing notice; do not include in an Apache-2.0 distribution. |

Add every new exception here and preserve its original notices. Third-party
dependency notices belong in release artifacts where the dependency license
requires them; this file is not a substitute for a dependency SBOM or notice
bundle.
