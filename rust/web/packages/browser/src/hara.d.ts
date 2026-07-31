export interface StartOptions {
  /** Override the default adjacent hara_wasm_bg.wasm URL. */
  wasmUrl?: RequestInfo | URL | ArrayBuffer | WebAssembly.Module | Uint8Array;
  /** Host resources registered before the first require. */
  resources?: Map<string, string> | Record<string, string>;
}

export interface HaraRuntime {
  eval(source: string): string;
  require(namespace: string): string;
  registerResource(namespace: string, source: string): void;
  evalInNamespace(namespace: string, source: string): string;
  currentNamespace(): string;
  dispose(): void;
  readonly raw: unknown;
}

export function start(options?: StartOptions): Promise<HaraRuntime>;
export const ready: Promise<HaraRuntime>;
export default start;
