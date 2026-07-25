/* tslint:disable */
/* eslint-disable */

export class PromiseHandle {
    free(): void;
    [Symbol.dispose](): void;
    adopt(other: PromiseHandle): boolean;
    constructor();
    reject(error: string): boolean;
    resolve(value: string): boolean;
    state(): string;
    value(): string;
}

export class Runtime {
    free(): void;
    [Symbol.dispose](): void;
    alias_namespace(alias: string, target: string): boolean;
    /**
     * Creates the portable L0 evaluator without loading the language-level
     * foundation. This is useful for small embedded surfaces whose commands
     * only require core forms and should become interactive immediately.
     */
    static core(): Runtime;
    create_namespace(name: string): boolean;
    current_namespace(): string;
    eval(source: string): string;
    /**
     * Evaluates source after selecting a namespace.
     */
    eval_in_namespace(name: string, source: string): string;
    eval_traced(source: string): string;
    extension_available(name: string): boolean;
    file_read(path: string): PromiseHandle;
    file_resolve(root: string, path: string): string;
    file_supported(): boolean;
    file_write(path: string, bytes: Uint8Array): PromiseHandle;
    /**
     * Returns whether a protocol method is registered in this runtime context.
     */
    has_protocol_method(protocol: string, method: string): boolean;
    /**
     * Installs the JS host handler that backs the `host/call` special form.
     */
    install_host_handler(handler: Function): void;
    install_loopback_socket_provider(): void;
    install_memory_file_provider(root: string): void;
    /**
     * Evaluates a registered resource in the current lexical namespace.
     */
    load_resource(name: string): string;
    constructor();
    /**
     * Registers a host-supplied Hara resource. Resources are source text, not executable host code.
     */
    register_resource(name: string, source: string): void;
    require_extension(name: string): string;
    /**
     * Loads a resource once; subsequent requires return the current loaded marker.
     */
    require_resource(name: string): string;
    require_resource_in_namespace(resource: string, namespace: string): string;
    resolve_namespace(name: string): string;
    socket_close(socket: bigint): void;
    /**
     * Opens a callback-based socket and returns its provider-owned handle.
     */
    socket_connect(host: string, port: number): bigint;
    socket_send(socket: bigint, bytes: Uint8Array): number;
    socket_supported(): boolean;
    use_namespace(name: string): boolean;
    visible_symbols(): string[];
}

export function init_wasm(): void;

export function target_profile(): string;

export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_promisehandle_free: (a: number, b: number) => void;
    readonly __wbg_runtime_free: (a: number, b: number) => void;
    readonly promisehandle_adopt: (a: number, b: number) => number;
    readonly promisehandle_new: () => number;
    readonly promisehandle_reject: (a: number, b: number, c: number) => number;
    readonly promisehandle_resolve: (a: number, b: number, c: number) => number;
    readonly promisehandle_state: (a: number) => [number, number];
    readonly promisehandle_value: (a: number) => [number, number, number, number];
    readonly runtime_alias_namespace: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly runtime_core: () => number;
    readonly runtime_create_namespace: (a: number, b: number, c: number) => number;
    readonly runtime_current_namespace: (a: number) => [number, number];
    readonly runtime_eval: (a: number, b: number, c: number) => [number, number, number, number];
    readonly runtime_eval_in_namespace: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly runtime_eval_traced: (a: number, b: number, c: number) => [number, number, number, number];
    readonly runtime_extension_available: (a: number, b: number, c: number) => number;
    readonly runtime_file_read: (a: number, b: number, c: number) => [number, number, number];
    readonly runtime_file_resolve: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly runtime_file_supported: (a: number) => number;
    readonly runtime_file_write: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly runtime_has_protocol_method: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly runtime_install_host_handler: (a: number, b: any) => void;
    readonly runtime_install_loopback_socket_provider: (a: number) => void;
    readonly runtime_install_memory_file_provider: (a: number, b: number, c: number) => void;
    readonly runtime_load_resource: (a: number, b: number, c: number) => [number, number, number, number];
    readonly runtime_new: () => number;
    readonly runtime_register_resource: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly runtime_require_extension: (a: number, b: number, c: number) => [number, number, number, number];
    readonly runtime_require_resource: (a: number, b: number, c: number) => [number, number, number, number];
    readonly runtime_require_resource_in_namespace: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly runtime_resolve_namespace: (a: number, b: number, c: number) => [number, number];
    readonly runtime_socket_close: (a: number, b: bigint) => [number, number];
    readonly runtime_socket_connect: (a: number, b: number, c: number, d: number) => [bigint, number, number];
    readonly runtime_socket_send: (a: number, b: bigint, c: number, d: number) => [number, number, number];
    readonly runtime_socket_supported: (a: number) => number;
    readonly runtime_use_namespace: (a: number, b: number, c: number) => number;
    readonly runtime_visible_symbols: (a: number) => [number, number];
    readonly target_profile: () => [number, number];
    readonly version: () => [number, number];
    readonly init_wasm: () => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
