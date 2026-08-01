#ifndef HOPLITE_HTA_H
#define HOPLITE_HTA_H

#include <ngx_config.h>
#include <ngx_core.h>
#include <ngx_http.h>

typedef enum {
    HOPLITE_HTA_NIL,
    HOPLITE_HTA_BOOL,
    HOPLITE_HTA_I64,
    HOPLITE_HTA_STRING,
    HOPLITE_HTA_BYTES,
    HOPLITE_HTA_KEYWORD,
    HOPLITE_HTA_VECTOR,
    HOPLITE_HTA_MAP
} hoplite_hta_kind_t;

typedef struct hoplite_hta_value hoplite_hta_value_t;

typedef struct {
    hoplite_hta_value_t *key;
    hoplite_hta_value_t *value;
} hoplite_hta_pair_t;

struct hoplite_hta_value {
    hoplite_hta_kind_t kind;
    union {
        ngx_flag_t boolean;
        int64_t i64;
        ngx_str_t text;
        struct {
            size_t count;
            hoplite_hta_value_t **items;
        } vector;
        struct {
            size_t count;
            hoplite_hta_pair_t *items;
        } map;
    } as;
};

ngx_int_t hoplite_hta_decode(ngx_pool_t *pool,
                             const u_char *data,
                             size_t len,
                             hoplite_hta_value_t **value);

ngx_int_t hoplite_hta_encode_request(ngx_http_request_t *request,
                                     ngx_str_t *output);

ngx_int_t hoplite_hta_encode_string(ngx_pool_t *pool,
                                    const ngx_str_t *value,
                                    ngx_str_t *output);

hoplite_hta_value_t *hoplite_hta_map_get(const hoplite_hta_value_t *map,
                                         const char *name);

ngx_int_t hoplite_hta_text(const hoplite_hta_value_t *value,
                           ngx_str_t *output);

ngx_int_t hoplite_hta_number(const hoplite_hta_value_t *value,
                             int64_t *output);

#endif
