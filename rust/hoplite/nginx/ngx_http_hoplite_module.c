#include <ngx_config.h>
#include <ngx_core.h>
#include <ngx_http.h>

#include "hoplite_hta.h"
#include "hoplite_runtime.h"

typedef struct {
    ngx_str_t bootstrap;
} ngx_http_hoplite_main_conf_t;

typedef struct {
    ngx_str_t handler;
} ngx_http_hoplite_loc_conf_t;

typedef struct ngx_http_hoplite_ctx_s ngx_http_hoplite_ctx_t;

typedef struct {
    ngx_event_t event;
    ngx_http_hoplite_ctx_t *ctx;
    uint64_t call;
} ngx_http_hoplite_sleep_t;

struct ngx_http_hoplite_ctx_s {
    ngx_queue_t queue;
    ngx_http_request_t *request;
    uint64_t work;
    ngx_flag_t queued;
    ngx_flag_t done;
    ngx_http_hoplite_sleep_t *sleep;
};

static hoplite_runtime_t *ngx_http_hoplite_runtime;
static ngx_queue_t ngx_http_hoplite_requests;
static ngx_flag_t ngx_http_hoplite_queue_ready;

static ngx_int_t ngx_http_hoplite_handler(ngx_http_request_t *request);
static char *ngx_http_hoplite_content(ngx_conf_t *cf, ngx_command_t *cmd,
                                      void *conf);
static void *ngx_http_hoplite_create_main_conf(ngx_conf_t *cf);
static void *ngx_http_hoplite_create_loc_conf(ngx_conf_t *cf);
static ngx_int_t ngx_http_hoplite_init_process(ngx_cycle_t *cycle);
static void ngx_http_hoplite_exit_process(ngx_cycle_t *cycle);
static void ngx_http_hoplite_cleanup(void *data);
static void ngx_http_hoplite_sleep_handler(ngx_event_t *event);
static ngx_int_t ngx_http_hoplite_drain(ngx_log_t *log);

static ngx_command_t ngx_http_hoplite_commands[] = {
    {
        ngx_string("hoplite_bootstrap"),
        NGX_HTTP_MAIN_CONF | NGX_CONF_TAKE1,
        ngx_conf_set_str_slot,
        NGX_HTTP_MAIN_CONF_OFFSET,
        offsetof(ngx_http_hoplite_main_conf_t, bootstrap),
        NULL
    },
    {
        ngx_string("hoplite_content"),
        NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1,
        ngx_http_hoplite_content,
        NGX_HTTP_LOC_CONF_OFFSET,
        0,
        NULL
    },
    ngx_null_command
};

static ngx_http_module_t ngx_http_hoplite_module_ctx = {
    NULL,
    NULL,
    ngx_http_hoplite_create_main_conf,
    NULL,
    NULL,
    NULL,
    ngx_http_hoplite_create_loc_conf,
    NULL
};

ngx_module_t ngx_http_hoplite_module = {
    NGX_MODULE_V1,
    &ngx_http_hoplite_module_ctx,
    ngx_http_hoplite_commands,
    NGX_HTTP_MODULE,
    NULL,
    NULL,
    ngx_http_hoplite_init_process,
    NULL,
    NULL,
    ngx_http_hoplite_exit_process,
    NULL,
    NGX_MODULE_V1_PADDING
};

static ngx_int_t
ngx_http_hoplite_copy(ngx_pool_t *pool, const ngx_str_t *source,
                      ngx_str_t *destination)
{
    destination->len = source->len;
    if (source->len == 0) {
        destination->data = NULL;
        return NGX_OK;
    }
    destination->data = ngx_pnalloc(pool, source->len);
    if (destination->data == NULL) {
        return NGX_ERROR;
    }
    ngx_memcpy(destination->data, source->data, source->len);
    return NGX_OK;
}

static ngx_http_hoplite_ctx_t *
ngx_http_hoplite_find(uint64_t work)
{
    ngx_queue_t *cursor;
    ngx_http_hoplite_ctx_t *ctx;

    if (!ngx_http_hoplite_queue_ready) {
        return NULL;
    }
    for (cursor = ngx_queue_head(&ngx_http_hoplite_requests);
         cursor != ngx_queue_sentinel(&ngx_http_hoplite_requests);
         cursor = ngx_queue_next(cursor))
    {
        ctx = ngx_queue_data(cursor, ngx_http_hoplite_ctx_t, queue);
        if (ctx->work == work) {
            return ctx;
        }
    }
    return NULL;
}

static void
ngx_http_hoplite_finish(ngx_http_hoplite_ctx_t *ctx)
{
    if (ctx->done) {
        return;
    }
    ctx->done = 1;
    if (ctx->queued) {
        ngx_queue_remove(&ctx->queue);
        ctx->queued = 0;
    }
    if (ngx_http_hoplite_runtime != NULL && ctx->work != 0) {
        (void) hoplite_work_close(ngx_http_hoplite_runtime, ctx->work);
    }
}

static ngx_int_t
ngx_http_hoplite_add_headers(ngx_http_request_t *request,
                             const hoplite_hta_value_t *headers)
{
    size_t i;
    ngx_str_t key, value, copied_key, copied_value;
    ngx_table_elt_t *header;
    hoplite_hta_pair_t *pair;

    if (headers == NULL || headers->kind != HOPLITE_HTA_MAP) {
        return NGX_OK;
    }

    for (i = 0; i < headers->as.map.count; i++) {
        pair = &headers->as.map.items[i];
        if (hoplite_hta_text(pair->key, &key) != NGX_OK
            || hoplite_hta_text(pair->value, &value) != NGX_OK)
        {
            continue;
        }

        if (key.len == sizeof("content-type") - 1
            && ngx_strncasecmp(key.data, (u_char *) "content-type", key.len) == 0)
        {
            if (ngx_http_hoplite_copy(request->pool, &value, &copied_value)
                != NGX_OK)
            {
                return NGX_ERROR;
            }
            request->headers_out.content_type = copied_value;
            continue;
        }

        if (key.len == sizeof("content-length") - 1
            && ngx_strncasecmp(key.data, (u_char *) "content-length", key.len) == 0)
        {
            continue;
        }

        header = ngx_list_push(&request->headers_out.headers);
        if (header == NULL
            || ngx_http_hoplite_copy(request->pool, &key, &copied_key) != NGX_OK
            || ngx_http_hoplite_copy(request->pool, &value, &copied_value) != NGX_OK)
        {
            return NGX_ERROR;
        }
        header->hash = 1;
        header->key = copied_key;
        header->value = copied_value;
    }
    return NGX_OK;
}

static void
ngx_http_hoplite_send(ngx_http_hoplite_ctx_t *ctx,
                      ngx_uint_t status,
                      const ngx_str_t *body,
                      const hoplite_hta_value_t *headers)
{
    ngx_http_request_t *request = ctx->request;
    ngx_buf_t *buffer;
    ngx_chain_t chain;
    ngx_int_t rc;
    ngx_str_t copied_body;

    if (ctx->done) {
        return;
    }
    if (ngx_http_hoplite_copy(request->pool, body, &copied_body) != NGX_OK
        || ngx_http_hoplite_add_headers(request, headers) != NGX_OK)
    {
        ngx_http_hoplite_finish(ctx);
        ngx_http_finalize_request(request, NGX_HTTP_INTERNAL_SERVER_ERROR);
        return;
    }

    request->headers_out.status = status;
    request->headers_out.content_length_n = (off_t) copied_body.len;
    if (request->headers_out.content_type.len == 0) {
        ngx_str_set(&request->headers_out.content_type, "text/plain");
    }

    rc = ngx_http_send_header(request);
    if (rc == NGX_ERROR || rc > NGX_OK || request->header_only
        || copied_body.len == 0)
    {
        ngx_http_hoplite_finish(ctx);
        ngx_http_finalize_request(request, rc);
        return;
    }

    buffer = ngx_calloc_buf(request->pool);
    if (buffer == NULL) {
        ngx_http_hoplite_finish(ctx);
        ngx_http_finalize_request(request, NGX_HTTP_INTERNAL_SERVER_ERROR);
        return;
    }

    buffer->pos = copied_body.data;
    buffer->last = copied_body.data + copied_body.len;
    buffer->memory = 1;
    buffer->last_buf = 1;
    chain.buf = buffer;
    chain.next = NULL;

    rc = ngx_http_output_filter(request, &chain);
    ngx_http_hoplite_finish(ctx);
    ngx_http_finalize_request(request, rc);
}

static void
ngx_http_hoplite_send_result(ngx_http_hoplite_ctx_t *ctx,
                             const hoplite_hta_value_t *payload)
{
    hoplite_hta_value_t *status_value;
    hoplite_hta_value_t *body_value;
    hoplite_hta_value_t *headers;
    int64_t status_number = NGX_HTTP_OK;
    ngx_str_t body = ngx_null_string;

    if (payload == NULL || payload->kind != HOPLITE_HTA_MAP) {
        ngx_str_set(&body, "Hoplite handler must return a response map\n");
        ngx_http_hoplite_send(ctx, NGX_HTTP_INTERNAL_SERVER_ERROR, &body, NULL);
        return;
    }

    status_value = hoplite_hta_map_get(payload, "status");
    body_value = hoplite_hta_map_get(payload, "body");
    headers = hoplite_hta_map_get(payload, "headers");

    if (status_value != NULL
        && hoplite_hta_number(status_value, &status_number) != NGX_OK)
    {
        status_number = NGX_HTTP_INTERNAL_SERVER_ERROR;
    }
    if (status_number < 100 || status_number > 599) {
        status_number = NGX_HTTP_INTERNAL_SERVER_ERROR;
    }
    if (body_value != NULL && body_value->kind != HOPLITE_HTA_NIL
        && hoplite_hta_text(body_value, &body) != NGX_OK)
    {
        ngx_str_set(&body, "Hoplite response body must be a string or bytes\n");
        status_number = NGX_HTTP_INTERNAL_SERVER_ERROR;
        headers = NULL;
    }

    ngx_http_hoplite_send(ctx, (ngx_uint_t) status_number, &body, headers);
}

static void
ngx_http_hoplite_send_error(ngx_http_hoplite_ctx_t *ctx,
                            const hoplite_hta_value_t *payload)
{
    hoplite_hta_value_t *message_value;
    ngx_str_t message;

    if (payload != NULL && payload->kind == HOPLITE_HTA_MAP) {
        message_value = hoplite_hta_map_get(payload, "message");
        if (message_value != NULL
            && hoplite_hta_text(message_value, &message) == NGX_OK)
        {
            ngx_log_error(NGX_LOG_ERR, ctx->request->connection->log, 0,
                          "hoplite handler error: %V", &message);
        }
    }

    ngx_str_set(&message, "Hoplite handler failed\n");
    ngx_http_hoplite_send(ctx, NGX_HTTP_INTERNAL_SERVER_ERROR, &message, NULL);
}

static ngx_int_t
ngx_http_hoplite_reject(uint64_t call, ngx_pool_t *pool, const char *message)
{
    ngx_str_t text, encoded;
    text.data = (u_char *) message;
    text.len = ngx_strlen(message);
    if (hoplite_hta_encode_string(pool, &text, &encoded) != NGX_OK) {
        return NGX_ERROR;
    }
    return hoplite_call_reject(ngx_http_hoplite_runtime, call,
                               encoded.data, encoded.len) == 0
        ? NGX_OK : NGX_ERROR;
}

static ngx_int_t
ngx_http_hoplite_host_call(hoplite_hta_value_t *event,
                           ngx_pool_t *pool,
                           ngx_log_t *log)
{
    int64_t call_number, work_number, delay;
    ngx_str_t service, method;
    hoplite_hta_value_t *args;
    ngx_http_hoplite_ctx_t *ctx;
    ngx_http_hoplite_sleep_t *sleep;

    if (event->as.vector.count != 8
        || hoplite_hta_number(event->as.vector.items[1], &call_number) != NGX_OK
        || hoplite_hta_number(event->as.vector.items[2], &work_number) != NGX_OK
        || hoplite_hta_text(event->as.vector.items[5], &service) != NGX_OK
        || hoplite_hta_text(event->as.vector.items[6], &method) != NGX_OK)
    {
        return NGX_ERROR;
    }

    ctx = ngx_http_hoplite_find((uint64_t) work_number);
    if (ctx == NULL || ctx->done) {
        return NGX_DECLINED;
    }

    if (service.len != sizeof("nginx") - 1
        || ngx_strncmp(service.data, "nginx", service.len) != 0
        || method.len != sizeof("sleep") - 1
        || ngx_strncmp(method.data, "sleep", method.len) != 0)
    {
        return ngx_http_hoplite_reject((uint64_t) call_number, pool,
                                       "unsupported Hoplite host call");
    }

    args = event->as.vector.items[7];
    if (args == NULL || args->kind != HOPLITE_HTA_VECTOR
        || args->as.vector.count != 1
        || hoplite_hta_number(args->as.vector.items[0], &delay) != NGX_OK
        || delay < 0 || delay > 3600000)
    {
        return ngx_http_hoplite_reject((uint64_t) call_number, pool,
                                       "nginx/sleep expects milliseconds from 0 to 3600000");
    }

    if (delay == 0) {
        return hoplite_call_resolve(ngx_http_hoplite_runtime,
                                    (uint64_t) call_number, NULL, 0) == 0
            ? NGX_OK : NGX_ERROR;
    }
    if (ctx->sleep != NULL) {
        return ngx_http_hoplite_reject((uint64_t) call_number, pool,
                                       "request already has a pending Hoplite operation");
    }

    sleep = ngx_pcalloc(ctx->request->pool, sizeof(*sleep));
    if (sleep == NULL) {
        return NGX_ERROR;
    }
    sleep->ctx = ctx;
    sleep->call = (uint64_t) call_number;
    sleep->event.handler = ngx_http_hoplite_sleep_handler;
    sleep->event.data = sleep;
    sleep->event.log = log;
    ctx->sleep = sleep;
    ngx_add_timer(&sleep->event, (ngx_msec_t) delay);
    return NGX_OK;
}

static ngx_int_t
ngx_http_hoplite_dispatch(hoplite_hta_value_t *event,
                          ngx_pool_t *pool,
                          ngx_log_t *log)
{
    int64_t kind, work;
    ngx_http_hoplite_ctx_t *ctx;

    if (event == NULL || event->kind != HOPLITE_HTA_VECTOR
        || event->as.vector.count < 3
        || hoplite_hta_number(event->as.vector.items[0], &kind) != NGX_OK)
    {
        return NGX_ERROR;
    }

    if (kind == 2) {
        return ngx_http_hoplite_host_call(event, pool, log);
    }

    if (event->as.vector.count != 3
        || hoplite_hta_number(event->as.vector.items[1], &work) != NGX_OK)
    {
        return NGX_ERROR;
    }
    ctx = ngx_http_hoplite_find((uint64_t) work);
    if (ctx == NULL || ctx->done) {
        return NGX_DECLINED;
    }

    if (kind == 0) {
        ngx_http_hoplite_send_result(ctx, event->as.vector.items[2]);
        return NGX_OK;
    }
    if (kind == 1) {
        ngx_http_hoplite_send_error(ctx, event->as.vector.items[2]);
        return NGX_OK;
    }
    return NGX_ERROR;
}

static ngx_int_t
ngx_http_hoplite_drain(ngx_log_t *log)
{
    hoplite_buffer_t buffer;
    hoplite_hta_value_t *event;
    ngx_pool_t *pool;
    ngx_int_t rc = NGX_OK;

    if (ngx_http_hoplite_runtime == NULL) {
        return NGX_ERROR;
    }

    while (hoplite_work_poll(ngx_http_hoplite_runtime) != 0) {
        buffer.data = NULL;
        buffer.len = 0;
        if (hoplite_work_next_event(ngx_http_hoplite_runtime, &buffer) != 0) {
            return NGX_ERROR;
        }

        pool = ngx_create_pool(4096, log);
        if (pool == NULL) {
            hoplite_buffer_free(buffer.data, buffer.len);
            return NGX_ERROR;
        }
        if (hoplite_hta_decode(pool, buffer.data, buffer.len, &event) != NGX_OK
            || ngx_http_hoplite_dispatch(event, pool, log) == NGX_ERROR)
        {
            ngx_log_error(NGX_LOG_ERR, log, 0,
                          "hoplite received an invalid runtime event");
            rc = NGX_ERROR;
        }
        ngx_destroy_pool(pool);
        hoplite_buffer_free(buffer.data, buffer.len);
    }
    return rc;
}

static void
ngx_http_hoplite_sleep_handler(ngx_event_t *event)
{
    ngx_http_hoplite_sleep_t *sleep = event->data;
    ngx_http_hoplite_ctx_t *ctx = sleep->ctx;

    if (ctx == NULL || ctx->done || ctx->sleep != sleep) {
        return;
    }
    ctx->sleep = NULL;
    if (hoplite_call_resolve(ngx_http_hoplite_runtime, sleep->call, NULL, 0) != 0
        || ngx_http_hoplite_drain(event->log) != NGX_OK)
    {
        ngx_str_t body = ngx_string("Hoplite runtime delivery failed\n");
        ngx_http_hoplite_send(ctx, NGX_HTTP_INTERNAL_SERVER_ERROR, &body, NULL);
    }
}

static void
ngx_http_hoplite_cleanup(void *data)
{
    ngx_http_hoplite_ctx_t *ctx = data;

    if (ctx->sleep != NULL && ctx->sleep->event.timer_set) {
        ngx_del_timer(&ctx->sleep->event);
    }
    ctx->sleep = NULL;

    if (!ctx->done && ngx_http_hoplite_runtime != NULL && ctx->work != 0) {
        (void) hoplite_work_cancel(ngx_http_hoplite_runtime, ctx->work);
        (void) hoplite_work_close(ngx_http_hoplite_runtime, ctx->work);
    }
    if (ctx->queued) {
        ngx_queue_remove(&ctx->queue);
        ctx->queued = 0;
    }
    ctx->done = 1;
}

static ngx_int_t
ngx_http_hoplite_handler(ngx_http_request_t *request)
{
    ngx_http_hoplite_loc_conf_t *conf;
    ngx_http_hoplite_ctx_t *ctx;
    ngx_pool_cleanup_t *cleanup;
    ngx_str_t binding, source;
    u_char *cursor;
    size_t source_len;

    if (ngx_http_hoplite_runtime == NULL) {
        return NGX_HTTP_SERVICE_UNAVAILABLE;
    }

    conf = ngx_http_get_module_loc_conf(request, ngx_http_hoplite_module);
    if (conf->handler.len == 0) {
        return NGX_DECLINED;
    }
    if (hoplite_hta_encode_request(request, &binding) != NGX_OK) {
        return NGX_HTTP_INTERNAL_SERVER_ERROR;
    }

    source_len = 1 + conf->handler.len + 1
               + sizeof("__hoplite_request") - 1 + 1;
    source.data = ngx_pnalloc(request->pool, source_len);
    if (source.data == NULL) {
        return NGX_HTTP_INTERNAL_SERVER_ERROR;
    }
    cursor = source.data;
    *cursor++ = '(';
    cursor = ngx_cpymem(cursor, conf->handler.data, conf->handler.len);
    *cursor++ = ' ';
    cursor = ngx_cpymem(cursor, "__hoplite_request",
                        sizeof("__hoplite_request") - 1);
    *cursor++ = ')';
    source.len = (size_t) (cursor - source.data);

    ctx = ngx_pcalloc(request->pool, sizeof(*ctx));
    if (ctx == NULL) {
        return NGX_HTTP_INTERNAL_SERVER_ERROR;
    }
    ctx->request = request;
    ngx_http_set_ctx(request, ctx, ngx_http_hoplite_module);

    cleanup = ngx_pool_cleanup_add(request->pool, 0);
    if (cleanup == NULL) {
        return NGX_HTTP_INTERNAL_SERVER_ERROR;
    }
    cleanup->handler = ngx_http_hoplite_cleanup;
    cleanup->data = ctx;

    ctx->work = hoplite_work_start(ngx_http_hoplite_runtime,
                              source.data, source.len,
                              binding.data, binding.len);
    if (ctx->work == 0) {
        return NGX_HTTP_INTERNAL_SERVER_ERROR;
    }

    ngx_queue_insert_tail(&ngx_http_hoplite_requests, &ctx->queue);
    ctx->queued = 1;
    request->main->count++;

    if (ngx_http_hoplite_drain(request->connection->log) != NGX_OK && !ctx->done) {
        ngx_str_t body = ngx_string("Hoplite runtime failed\n");
        ngx_http_hoplite_send(ctx, NGX_HTTP_INTERNAL_SERVER_ERROR, &body, NULL);
    }
    return NGX_DONE;
}

static ngx_int_t
ngx_http_hoplite_read_file(ngx_cycle_t *cycle, const ngx_str_t *path,
                           ngx_str_t *source)
{
    ngx_file_t file;
    ngx_file_info_t info;
    ssize_t read;

    ngx_memzero(&file, sizeof(file));
    file.name = *path;
    file.log = cycle->log;
    file.fd = ngx_open_file(path->data, NGX_FILE_RDONLY, NGX_FILE_OPEN, 0);
    if (file.fd == NGX_INVALID_FILE) {
        ngx_log_error(NGX_LOG_EMERG, cycle->log, ngx_errno,
                      "hoplite could not open bootstrap file %V", path);
        return NGX_ERROR;
    }
    if (ngx_fd_info(file.fd, &info) == NGX_FILE_ERROR) {
        ngx_close_file(file.fd);
        return NGX_ERROR;
    }

    source->len = (size_t) ngx_file_size(&info);
    source->data = ngx_alloc(source->len + sizeof("\nnil") - 1, cycle->log);
    if (source->data == NULL) {
        ngx_close_file(file.fd);
        return NGX_ERROR;
    }
    read = ngx_read_file(&file, source->data, source->len, 0);
    ngx_close_file(file.fd);
    if (read == NGX_ERROR || (size_t) read != source->len) {
        ngx_free(source->data);
        source->data = NULL;
        return NGX_ERROR;
    }
    ngx_memcpy(source->data + source->len, "\nnil", sizeof("\nnil") - 1);
    source->len += sizeof("\nnil") - 1;
    return NGX_OK;
}

static ngx_int_t
ngx_http_hoplite_bootstrap(ngx_cycle_t *cycle, const ngx_str_t *path)
{
    ngx_str_t source;
    hoplite_buffer_t buffer;
    hoplite_hta_value_t *event;
    ngx_pool_t *pool;
    int64_t kind, event_work;
    uint64_t work;
    ngx_int_t rc = NGX_ERROR;

    if (ngx_http_hoplite_read_file(cycle, path, &source) != NGX_OK) {
        return NGX_ERROR;
    }
    work = hoplite_work_start(ngx_http_hoplite_runtime,
                         source.data, source.len, NULL, 0);
    ngx_free(source.data);
    if (work == 0 || hoplite_work_poll(ngx_http_hoplite_runtime) == 0) {
        ngx_log_error(NGX_LOG_EMERG, cycle->log, 0,
                      "hoplite bootstrap suspended or failed to start");
        return NGX_ERROR;
    }

    while (hoplite_work_poll(ngx_http_hoplite_runtime) != 0) {
        if (hoplite_work_next_event(ngx_http_hoplite_runtime, &buffer) != 0) {
            break;
        }
        pool = ngx_create_pool(4096, cycle->log);
        if (pool == NULL) {
            hoplite_buffer_free(buffer.data, buffer.len);
            break;
        }
        if (hoplite_hta_decode(pool, buffer.data, buffer.len, &event) == NGX_OK
            && event->kind == HOPLITE_HTA_VECTOR
            && event->as.vector.count == 3
            && hoplite_hta_number(event->as.vector.items[0], &kind) == NGX_OK
            && hoplite_hta_number(event->as.vector.items[1], &event_work) == NGX_OK
            && (uint64_t) event_work == work)
        {
            rc = kind == 0 ? NGX_OK : NGX_ERROR;
        }
        ngx_destroy_pool(pool);
        hoplite_buffer_free(buffer.data, buffer.len);
    }
    (void) hoplite_work_close(ngx_http_hoplite_runtime, work);

    if (rc != NGX_OK) {
        ngx_log_error(NGX_LOG_EMERG, cycle->log, 0,
                      "hoplite bootstrap evaluation failed");
    }
    return rc;
}

static ngx_int_t
ngx_http_hoplite_init_process(ngx_cycle_t *cycle)
{
    ngx_http_hoplite_main_conf_t *conf;

    ngx_queue_init(&ngx_http_hoplite_requests);
    ngx_http_hoplite_queue_ready = 1;
    ngx_http_hoplite_runtime = hoplite_runtime_new();
    if (ngx_http_hoplite_runtime == NULL || hoplite_abi_version() != 1) {
        ngx_log_error(NGX_LOG_EMERG, cycle->log, 0,
                      "hoplite runtime could not be initialized");
        return NGX_ERROR;
    }

    conf = ngx_http_cycle_get_module_main_conf(cycle, ngx_http_hoplite_module);
    if (conf != NULL && conf->bootstrap.len != 0
        && ngx_http_hoplite_bootstrap(cycle, &conf->bootstrap) != NGX_OK)
    {
        return NGX_ERROR;
    }
    return NGX_OK;
}

static void
ngx_http_hoplite_exit_process(ngx_cycle_t *cycle)
{
    (void) cycle;
    if (ngx_http_hoplite_runtime != NULL) {
        hoplite_runtime_free(ngx_http_hoplite_runtime);
        ngx_http_hoplite_runtime = NULL;
    }
    ngx_http_hoplite_queue_ready = 0;
}

static void *
ngx_http_hoplite_create_main_conf(ngx_conf_t *cf)
{
    return ngx_pcalloc(cf->pool, sizeof(ngx_http_hoplite_main_conf_t));
}

static void *
ngx_http_hoplite_create_loc_conf(ngx_conf_t *cf)
{
    return ngx_pcalloc(cf->pool, sizeof(ngx_http_hoplite_loc_conf_t));
}

static char *
ngx_http_hoplite_content(ngx_conf_t *cf, ngx_command_t *cmd, void *conf)
{
    ngx_http_hoplite_loc_conf_t *location = conf;
    ngx_http_core_loc_conf_t *core;
    ngx_str_t *value;

    (void) cmd;
    if (location->handler.len != 0) {
        return "is duplicate";
    }
    value = cf->args->elts;
    location->handler = value[1];

    core = ngx_http_conf_get_module_loc_conf(cf, ngx_http_core_module);
    core->handler = ngx_http_hoplite_handler;
    return NGX_CONF_OK;
}
