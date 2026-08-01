#ifndef OPENHAL_HTA_H
#define OPENHAL_HTA_H

#include <ngx_config.h>
#include <ngx_core.h>
#include <ngx_http.h>

typedef enum {
    OPENHAL_HTA_NIL,
    OPENHAL_HTA_BOOL,
    OPENHAL_HTA_I64,
    OPENHAL_HTA_STRING,
    OPENHAL_HTA_BYTES,
    OPENHAL_HTA_KEYWORD,
    OPENHAL_HTA_VECTOR,
    OPENHAL_HTA_MAP
} openhal_hta_kind_t;

typedef struct openhal_hta_value openhal_hta_value_t;

typedef struct {
    openhal_hta_value_t *key;
    openhal_hta_value_t *value;
} openhal_hta_pair_t;

struct openhal_hta_value {
    openhal_hta_kind_t kind;
    union {
        ngx_flag_t boolean;
        int64_t i64;
        ngx_str_t text;
        struct {
            size_t count;
            openhal_hta_value_t **items;
        } vector;
        struct {
            size_t count;
            openhal_hta_pair_t *items;
        } map;
    } as;
};

ngx_int_t openhal_hta_decode(ngx_pool_t *pool,
                             const u_char *data,
                             size_t len,
                             openhal_hta_value_t **value);

ngx_int_t openhal_hta_encode_request(ngx_http_request_t *request,
                                     ngx_str_t *output);

ngx_int_t openhal_hta_encode_string(ngx_pool_t *pool,
                                    const ngx_str_t *value,
                                    ngx_str_t *output);

openhal_hta_value_t *openhal_hta_map_get(const openhal_hta_value_t *map,
                                         const char *name);

ngx_int_t openhal_hta_text(const openhal_hta_value_t *value,
                           ngx_str_t *output);

ngx_int_t openhal_hta_number(const openhal_hta_value_t *value,
                             int64_t *output);

#endif
