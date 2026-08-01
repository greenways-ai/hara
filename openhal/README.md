# OpenHAL

OpenHAL embeds the Hara runtime in an Nginx worker and maps Nginx asynchronous operations onto Hara promises and coroutine suspension.

This first implementation proves two paths:

1. A synchronous HAL function receives an HTTP request map and returns a response map.
2. `std.native.Host/call` yields a Hara promise, `std.foundation.coroutine/await` suspends the evaluator fiber, and an Nginx timer resolves that promise without blocking the worker.

## Architecture

```text
HTTP request
    -> ngx_http_openhal_module
    -> HTA request value
    -> worker-local openhal-runtime
    -> HAL handler
       -> response map, or
       -> Host/call("nginx", "sleep", [milliseconds])
            -> unresolved Hara Promise
            -> suspended EvalFiber
            -> ngx_event_t timer
            -> openhal_deliver(...)
            -> Promise resolution
            -> resumed EvalFiber
    -> HTA response value
    -> Nginx response
```

There is one `OpenHalRuntime` per Nginx worker. Runtime values, promises, fibers, and host calls never cross worker boundaries.

## Nginx configuration

```nginx
load_module modules/ngx_http_openhal_module.so;

events {}

http {
    openhal_bootstrap /etc/openhal/app.hal;

    server {
        listen 8080;

        location /hello {
            openhal_content openhal.app/hello;
        }

        location /delay {
            openhal_content openhal.app/delayed;
        }
    }
}
```

`openhal_bootstrap` is evaluated once during each worker's `init_process` lifecycle. It must complete synchronously. `openhal_content` identifies a function loaded by that bootstrap source.

## HAL handlers

```clojure
(ns openhal.app)

(defn hello [request]
  {:status 200
   :headers {"content-type" "text/plain; charset=utf-8"}
   :body "Hello from OpenHAL\n"})

(defn delayed [request]
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
docker build -f openhal/docker/Dockerfile -t openhal .
docker run --rm -p 8080:8080 openhal
```

Then:

```bash
curl -i http://localhost:8080/hello
curl -i http://localhost:8080/delay
curl -i http://localhost:8080/inspect?sample=true
```

## Native build

Build the worker runtime:

```bash
make -C openhal runtime
```

Build the dynamic module against an unpacked Nginx source tree:

```bash
make -C openhal module NGINX_SRC=/path/to/nginx-source
```

The build expects `libopenhal_runtime.so` in `openhal/runtime/target/release`. At runtime, install it in a system library directory or configure the dynamic loader path.

## Runtime ABI

The Rust library exposes a native-safe C ABI:

```c
openhal_runtime_t *openhal_runtime_new(void);
uint64_t openhal_start(...);
size_t openhal_poll(...);
int openhal_next_event(...);
int openhal_deliver(...);
int openhal_cancel(...);
```

Unlike the existing 32-bit-oriented raw HTA pointer packing, `openhal_next_event` returns pointer and length through an explicit `openhal_buffer_t`, making the bridge safe for native 64-bit Nginx processes.

Events retain Hara's HTA envelope:

```text
[0 task result]                                      completion
[1 task error]                                       failure
[2 call task "OPENHAL" nil service method arguments] host request
```

## Current boundary

This is intentionally a proof implementation. It supports:

- one Hara runtime per worker;
- bootstrap evaluation;
- request maps;
- response status, headers, strings, and bytes;
- coroutine suspension over a host promise;
- `nginx/sleep` through `ngx_event_t`;
- request cancellation and task cleanup;
- Nginx configuration reload through normal worker replacement.

The next host adapters should be request-body reading and Nginx subrequests. Both can use the same `Host/call -> Promise -> EvalFiber::Suspended -> openhal_deliver` path demonstrated by the timer.
