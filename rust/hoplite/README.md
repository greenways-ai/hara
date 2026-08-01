# Hoplite

Hoplite embeds the Hara runtime in an Nginx worker and maps Nginx asynchronous operations onto Hara promises and coroutine suspension.

The implementation supports two request paths:

1. A synchronous HAL function receives an HTTP request map and returns a response map.
2. `std.native.Host/call` yields a Hara promise, `std.foundation.coroutine/await` suspends the bytecode VM fiber, and an Nginx timer resolves that promise without blocking the worker.

## Architecture

```text
HTTP request
    -> ngx_http_hoplite_module
    -> HTA request value
    -> worker-local Hoplite WorkRuntime
    -> HAL handler
       -> response map, or
       -> Host/call("nginx", "sleep", [milliseconds])
            -> unresolved Hara Promise
            -> suspended VM fiber
            -> ngx_event_t timer
            -> hoplite_call_resolve/hoplite_call_reject(...)
            -> Promise resolution
            -> queued fiber resumption
    -> HTA response value
    -> Nginx response
```

There is one `HopliteRuntime` per Nginx worker. Runtime values, promises, fibers, and host calls never cross worker boundaries.

## Nginx configuration

```nginx
events {}

http {
    hoplite_bootstrap /etc/hoplite/app.hal;

    server {
        listen 8080;

        location /hello {
            hoplite_content hoplite.app/hello;
        }

        location /delay {
            hoplite_content hoplite.app/delayed;
        }
    }
}
```

`hoplite_bootstrap` is evaluated once during each worker's `init_process` lifecycle. It must complete synchronously. `hoplite_content` identifies a function loaded by that bootstrap source.

## HAL handlers

```clojure
(ns hoplite.app)

(defn hello [request]
  {:status 200
   :headers {"content-type" "text/plain; charset=utf-8"}
   :body "Hello from Hoplite\n"})

(defn ^:async delayed [request]
  (std.foundation.coroutine/await
    (std.native.Host/call "nginx" "sleep" [25]))
  {:status 200
   :headers {"content-type" "text/plain; charset=utf-8"}
   :body "resumed\n"})
```

The request value currently contains:

```clojure
{:method "GET"
 :uri "/inspect?a=1"
 :path "/inspect"
 :query-string "a=1"
 :remote-address "127.0.0.1"
 :headers {"Host" "localhost:8080"}}
```

A handler returns:

```clojure
{:status 200
 :headers {"content-type" "text/plain"}
 :body "hello"}
```

The response body may be a Hara string or byte value.

## Run the Docker experiment

From the repository root:

```bash
docker build -f rust/hoplite/docker/Dockerfile -t hoplite .
docker run --rm -p 8080:8080 hoplite
```

Then:

```bash
curl -i http://localhost:8080/hello
curl -i http://localhost:8080/delay
curl -i http://localhost:8080/inspect?sample=true
```

## Native build

On macOS, install the native dependencies first:

```bash
brew install openssl@3 pcre2
```

Build the worker runtime:

```bash
make -C rust/hoplite runtime
```

Build the pinned Nginx distribution with Hoplite statically built in:

```bash
make -C rust/hoplite nginx NGINX_SRC=/path/to/nginx-1.30.4
```

Build the standalone Hoplite executable:

```bash
make -C rust/hoplite macos
```

`osx` is an alias for `macos`. The resulting
`target/dist/hoplite-v<version>-<target>` is the complete distribution: the
Hara runtime and statically linked Hoplite Nginx host are embedded in that one
executable. The Makefile downloads and verifies the pinned Nginx source, then
prints the executable checksum and installation instructions. Set
`NGINX_SRC=/path/to/nginx-1.30.4` only to override the downloaded source tree.

The build uses Homebrew OpenSSL and PCRE2 static archives while compiling, but
the resulting executable has no Homebrew runtime-library dependency. There is
no loadable Hoplite module, separate Nginx executable, or runtime shared
library to deploy.

## CLI

Hoplite is the Greenways-packaged Hara runtime with the Nginx host embedded.
It preserves the full Hara CLI and has exactly the same Hara runtime version:

```bash
cargo build --manifest-path rust/Cargo.toml --release --features hoplite \
  --bin hara --bin hoplite
hoplite eval '(+ 19 23)'
hoplite run app.hal
hoplite project check
hoplite package --help
hoplite serve
hoplite serve install /path/to/project
hoplite serve status /path/to/project
hoplite serve uninstall /path/to/project
```

`hoplite serve build` emits `.hoplite/app.hal`, the HBC2 bytecode artifact
`.hoplite/app.hbc`, and a generated Nginx configuration. `server.edn` controls
`:hoplite/listen` and `:hoplite/workers`; `routes.edn` maps route `:path` values
to Hara `:handler` vars. `hoplite serve foreground` keeps the embedded host in
the foreground for service managers. On macOS,
`hoplite serve install /path/to/project` creates and loads a per-user
LaunchAgent; `hoplite serve uninstall /path/to/project` unloads and removes it.
Set `HOPLITE_NGINX` only when deliberately overriding the embedded host for
development.

## Runtime ABI

The Rust library exposes a native-safe C ABI:

```c
hoplite_runtime_t *hoplite_runtime_new(void);
uint64_t hoplite_work_start(...);
size_t hoplite_work_poll(...);
int hoplite_work_next_event(...);
int hoplite_work_send(...);
int hoplite_call_resolve(...);
int hoplite_call_reject(...);
int hoplite_work_cancel(...);
int hoplite_work_close(...);
```

Unlike the existing 32-bit-oriented raw HTA pointer packing, `hoplite_work_next_event` returns pointer and length through an explicit `hoplite_buffer_t`, making the bridge safe for native 64-bit Nginx processes.

Events retain Hara's HTA envelope:

```text
[0 work result]                                      completion
[1 work error]                                       failure
[2 call work "HOPLITE" nil service method arguments] host request
```

## Current boundary

The current boundary supports:

- one Hara runtime per worker;
- bootstrap evaluation;
- request maps;
- response status, headers, strings, and bytes;
- coroutine suspension over a host promise;
- `nginx/sleep` through `ngx_event_t`;
- request cancellation and work cleanup;
- Nginx configuration reload through normal worker replacement.

Request-body reading, subrequests, upstreams, streaming, and WebSocket events
remain host-adapter work. They use the same `Host/call -> Promise -> suspended
fiber -> hoplite_call_resolve/hoplite_call_reject` path demonstrated by the
timer; none of them belongs inside `Promise`.
