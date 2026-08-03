# @hara-lang/visual-language

Hara's shared precision-material design system. It contains brand tokens,
cross-domain light/dark theme handling, responsive motifs, and small Astro
primitives. Interactive editors and workbenches remain in `@hara-lang/ui`.

Consumers should pin a release tag exactly:

```json
"@hara-lang/visual-language": "github:hara-lang/visual-language#v1.0.0"
```

Import `theme.css`, `motifs.css`, and `theme.js` before using the Astro
components. Edge, Aperture, and Rack use resolution-independent CSS geometry;
the generated contact sheet is retained only as an art-direction reference.
The theme script stores `system`, `light`, or `dark` in the `hara-theme` cookie
on `hara-lang.org` and uses local storage on local hosts.
