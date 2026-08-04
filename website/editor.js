// The Paredit implementation moved to the @hara-lang/live package
// (website/packages/live/src/editor.js). This shim keeps existing importers
// (app.js, editor.test.mjs) working unchanged.
export * from "./packages/live/src/editor.js";
