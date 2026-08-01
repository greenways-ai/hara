#include "hoplite_hta.h"

#define OH_NIL 0
#define OH_FALSE 1
#define OH_TRUE 2
#define OH_I64 3
#define OH_STRING 4
#define OH_BYTES 5
#define OH_KEYWORD 6
#define OH_VECTOR 9
#define OH_MAP 11

static const u_char hoplite_magic[] = {'H', 'T', 'A', '1'};

typedef struct {
    ngx_pool_t *pool;
    const u_char *data;
    size_t len;
    size_t cursor;
} hoplite_reader_t;

typedef struct {
    u_char *data;
    size_t len;
    size_t cursor;
} hoplite_writer_t;

static ngx_int_t hoplite_read_value(hoplite_reader_t *reader,
                                    hoplite_hta_value_t **output);

static ngx_int_t
hoplite_take(hoplite_reader_t *reader, size_t size, const u_char **output)
{
    if (size > reader->len || reader->cursor > reader->len - size) {
        return NGX_ERROR;
    }
    *output = reader->data + reader->cursor;
    reader->cursor += size;
    return NGX_OK;
}

static ngx_int_t
hoplite_read_u32(hoplite_reader_t *reader, uint32_t *output)
{
    const u_char *data;
    if (hoplite_take(reader, 4, &data) != NGX_OK) {
        return NGX_ERROR;
    }
    *output = ((uint32_t) data[0] << 24)
            | ((uint32_t) data[1] << 16)
            | ((uint32_t) data[2] << 8)
            | (uint32_t) data[3];
    return NGX_OK;
}

static ngx_int_t
hoplite_read_text(hoplite_reader_t *reader, ngx_str_t *output)
{
    uint32_t length;
    const u_char *data;
    if (hoplite_read_u32(reader, &length) != NGX_OK
        || hoplite_take(reader, length, &data) != NGX_OK)
    {
        return NGX_ERROR;
    }
    output->data = (u_char *) data;
    output->len = length;
    return NGX_OK;
}

static hoplite_hta_value_t *
hoplite_new_value(hoplite_reader_t *reader, hoplite_hta_kind_t kind)
{
    hoplite_hta_value_t *value = ngx_pcalloc(reader->pool, sizeof(*value));
    if (value != NULL) {
        value->kind = kind;
    }
    return value;
}

static ngx_int_t
hoplite_read_sequence(hoplite_reader_t *reader, hoplite_hta_value_t *value)
{
    uint32_t count;
    size_t i;

    if (hoplite_read_u32(reader, &count) != NGX_OK) {
        return NGX_ERROR;
    }
    if (count > 100000) {
        return NGX_ERROR;
    }

    value->as.vector.count = count;
    value->as.vector.items = ngx_pcalloc(
        reader->pool, sizeof(hoplite_hta_value_t *) * count);
    if (count != 0 && value->as.vector.items == NULL) {
        return NGX_ERROR;
    }

    for (i = 0; i < count; i++) {
        if (hoplite_read_value(reader, &value->as.vector.items[i]) != NGX_OK) {
            return NGX_ERROR;
        }
    }
    return NGX_OK;
}

static ngx_int_t
hoplite_read_map(hoplite_reader_t *reader, hoplite_hta_value_t *value)
{
    uint32_t count;
    size_t i;

    if (hoplite_read_u32(reader, &count) != NGX_OK) {
        return NGX_ERROR;
    }
    if (count > 100000) {
        return NGX_ERROR;
    }

    value->as.map.count = count;
    value->as.map.items = ngx_pcalloc(
        reader->pool, sizeof(hoplite_hta_pair_t) * count);
    if (count != 0 && value->as.map.items == NULL) {
        return NGX_ERROR;
    }

    for (i = 0; i < count; i++) {
        if (hoplite_read_value(reader, &value->as.map.items[i].key) != NGX_OK
            || hoplite_read_value(reader, &value->as.map.items[i].value) != NGX_OK)
        {
            return NGX_ERROR;
        }
    }
    return NGX_OK;
}

static ngx_int_t
hoplite_read_value(hoplite_reader_t *reader, hoplite_hta_value_t **output)
{
    const u_char *tag_data;
    const u_char *number;
    hoplite_hta_value_t *value;
    uint64_t raw;
    ngx_uint_t tag;

    if (hoplite_take(reader, 1, &tag_data) != NGX_OK) {
        return NGX_ERROR;
    }
    tag = tag_data[0];

    switch (tag) {
    case OH_NIL:
        value = hoplite_new_value(reader, HOPLITE_HTA_NIL);
        break;
    case OH_FALSE:
    case OH_TRUE:
        value = hoplite_new_value(reader, HOPLITE_HTA_BOOL);
        if (value != NULL) {
            value->as.boolean = tag == OH_TRUE;
        }
        break;
    case OH_I64:
        value = hoplite_new_value(reader, HOPLITE_HTA_I64);
        if (value == NULL || hoplite_take(reader, 8, &number) != NGX_OK) {
            return NGX_ERROR;
        }
        raw = ((uint64_t) number[0] << 56)
            | ((uint64_t) number[1] << 48)
            | ((uint64_t) number[2] << 40)
            | ((uint64_t) number[3] << 32)
            | ((uint64_t) number[4] << 24)
            | ((uint64_t) number[5] << 16)
            | ((uint64_t) number[6] << 8)
            | (uint64_t) number[7];
        value->as.i64 = (int64_t) raw;
        break;
    case OH_STRING:
    case OH_BYTES:
    case OH_KEYWORD:
        value = hoplite_new_value(
            reader,
            tag == OH_STRING ? HOPLITE_HTA_STRING
            : tag == OH_BYTES ? HOPLITE_HTA_BYTES
                              : HOPLITE_HTA_KEYWORD);
        if (value == NULL || hoplite_read_text(reader, &value->as.text) != NGX_OK) {
            return NGX_ERROR;
        }
        break;
    case OH_VECTOR:
        value = hoplite_new_value(reader, HOPLITE_HTA_VECTOR);
        if (value == NULL || hoplite_read_sequence(reader, value) != NGX_OK) {
            return NGX_ERROR;
        }
        break;
    case OH_MAP:
        value = hoplite_new_value(reader, HOPLITE_HTA_MAP);
        if (value == NULL || hoplite_read_map(reader, value) != NGX_OK) {
            return NGX_ERROR;
        }
        break;
    default:
        return NGX_ERROR;
    }

    if (value == NULL) {
        return NGX_ERROR;
    }
    *output = value;
    return NGX_OK;
}

ngx_int_t
hoplite_hta_decode(ngx_pool_t *pool, const u_char *data, size_t len,
                   hoplite_hta_value_t **value)
{
    hoplite_reader_t reader;

    if (len < sizeof(hoplite_magic)
        || ngx_memcmp(data, hoplite_magic, sizeof(hoplite_magic)) != 0)
    {
        return NGX_ERROR;
    }

    reader.pool = pool;
    reader.data = data;
    reader.len = len;
    reader.cursor = sizeof(hoplite_magic);

    if (hoplite_read_value(&reader, value) != NGX_OK || reader.cursor != len) {
        return NGX_ERROR;
    }
    return NGX_OK;
}

static ngx_int_t
hoplite_write(hoplite_writer_t *writer, const void *data, size_t len)
{
    if (len > writer->len || writer->cursor > writer->len - len) {
        return NGX_ERROR;
    }
    ngx_memcpy(writer->data + writer->cursor, data, len);
    writer->cursor += len;
    return NGX_OK;
}

static ngx_int_t
hoplite_write_byte(hoplite_writer_t *writer, u_char value)
{
    return hoplite_write(writer, &value, 1);
}

static ngx_int_t
hoplite_write_u32(hoplite_writer_t *writer, uint32_t value)
{
    u_char bytes[4];
    bytes[0] = (u_char) (value >> 24);
    bytes[1] = (u_char) (value >> 16);
    bytes[2] = (u_char) (value >> 8);
    bytes[3] = (u_char) value;
    return hoplite_write(writer, bytes, sizeof(bytes));
}

static ngx_int_t
hoplite_write_text(hoplite_writer_t *writer, u_char tag, const ngx_str_t *value)
{
    if (value->len > UINT32_MAX
        || hoplite_write_byte(writer, tag) != NGX_OK
        || hoplite_write_u32(writer, (uint32_t) value->len) != NGX_OK)
    {
        return NGX_ERROR;
    }
    return hoplite_write(writer, value->data, value->len);
}

static ngx_int_t
hoplite_write_pair(hoplite_writer_t *writer, const char *key,
                   const ngx_str_t *value)
{
    ngx_str_t name;
    name.data = (u_char *) key;
    name.len = ngx_strlen(key);
    return hoplite_write_text(writer, OH_KEYWORD, &name) == NGX_OK
        && hoplite_write_text(writer, OH_STRING, value) == NGX_OK
        ? NGX_OK : NGX_ERROR;
}

static size_t
hoplite_request_capacity(ngx_http_request_t *request)
{
    size_t capacity = 1024 + request->request_line.len + request->unparsed_uri.len
                    + request->uri.len + request->args.len
                    + request->connection->addr_text.len;
    ngx_list_part_t *part = &request->headers_in.headers.part;
    ngx_table_elt_t *header = part->elts;
    ngx_uint_t i;

    for (i = 0; ; i++) {
        if (i >= part->nelts) {
            if (part->next == NULL) {
                break;
            }
            part = part->next;
            header = part->elts;
            i = 0;
        }
        capacity += header[i].key.len + header[i].value.len + 16;
    }
    return capacity;
}

ngx_int_t
hoplite_hta_encode_request(ngx_http_request_t *request, ngx_str_t *output)
{
    hoplite_writer_t writer;
    ngx_list_part_t *part;
    ngx_table_elt_t *header;
    ngx_uint_t i, header_count = 0;
    ngx_str_t method, uri, path, args, remote;
    size_t capacity = hoplite_request_capacity(request);

    output->data = ngx_pnalloc(request->pool, capacity);
    if (output->data == NULL) {
        return NGX_ERROR;
    }
    output->len = 0;
    writer.data = output->data;
    writer.len = capacity;
    writer.cursor = 0;

    method = request->method_name;
    uri = request->unparsed_uri;
    path = request->uri;
    args = request->args;
    remote = request->connection->addr_text;

    part = &request->headers_in.headers.part;
    header = part->elts;
    for (i = 0; ; i++) {
        if (i >= part->nelts) {
            if (part->next == NULL) {
                break;
            }
            part = part->next;
            header = part->elts;
            i = 0;
        }
        header_count++;
    }

    if (hoplite_write(&writer, hoplite_magic, sizeof(hoplite_magic)) != NGX_OK
        || hoplite_write_byte(&writer, OH_MAP) != NGX_OK
        || hoplite_write_u32(&writer, 6) != NGX_OK
        || hoplite_write_pair(&writer, "method", &method) != NGX_OK
        || hoplite_write_pair(&writer, "uri", &uri) != NGX_OK
        || hoplite_write_pair(&writer, "path", &path) != NGX_OK
        || hoplite_write_pair(&writer, "query-string", &args) != NGX_OK
        || hoplite_write_pair(&writer, "remote-address", &remote) != NGX_OK)
    {
        return NGX_ERROR;
    }

    {
        ngx_str_t headers_key = ngx_string("headers");
        if (hoplite_write_text(&writer, OH_KEYWORD, &headers_key) != NGX_OK
            || hoplite_write_byte(&writer, OH_MAP) != NGX_OK
            || hoplite_write_u32(&writer, (uint32_t) header_count) != NGX_OK)
        {
            return NGX_ERROR;
        }
    }

    part = &request->headers_in.headers.part;
    header = part->elts;
    for (i = 0; ; i++) {
        if (i >= part->nelts) {
            if (part->next == NULL) {
                break;
            }
            part = part->next;
            header = part->elts;
            i = 0;
        }
        if (hoplite_write_text(&writer, OH_STRING, &header[i].key) != NGX_OK
            || hoplite_write_text(&writer, OH_STRING, &header[i].value) != NGX_OK)
        {
            return NGX_ERROR;
        }
    }

    output->len = writer.cursor;
    return NGX_OK;
}

ngx_int_t
hoplite_hta_encode_string(ngx_pool_t *pool, const ngx_str_t *value,
                          ngx_str_t *output)
{
    hoplite_writer_t writer;
    size_t capacity = sizeof(hoplite_magic) + 1 + 4 + value->len;

    output->data = ngx_pnalloc(pool, capacity);
    if (output->data == NULL) {
        return NGX_ERROR;
    }
    writer.data = output->data;
    writer.len = capacity;
    writer.cursor = 0;

    if (hoplite_write(&writer, hoplite_magic, sizeof(hoplite_magic)) != NGX_OK
        || hoplite_write_text(&writer, OH_STRING, value) != NGX_OK)
    {
        return NGX_ERROR;
    }
    output->len = writer.cursor;
    return NGX_OK;
}

ngx_int_t
hoplite_hta_text(const hoplite_hta_value_t *value, ngx_str_t *output)
{
    if (value == NULL
        || (value->kind != HOPLITE_HTA_STRING
            && value->kind != HOPLITE_HTA_BYTES
            && value->kind != HOPLITE_HTA_KEYWORD))
    {
        return NGX_ERROR;
    }
    *output = value->as.text;
    return NGX_OK;
}

ngx_int_t
hoplite_hta_number(const hoplite_hta_value_t *value, int64_t *output)
{
    if (value == NULL || value->kind != HOPLITE_HTA_I64) {
        return NGX_ERROR;
    }
    *output = value->as.i64;
    return NGX_OK;
}

hoplite_hta_value_t *
hoplite_hta_map_get(const hoplite_hta_value_t *map, const char *name)
{
    size_t length = ngx_strlen(name);
    size_t i;
    hoplite_hta_value_t *key;

    if (map == NULL || map->kind != HOPLITE_HTA_MAP) {
        return NULL;
    }

    for (i = 0; i < map->as.map.count; i++) {
        key = map->as.map.items[i].key;
        if (key != NULL
            && (key->kind == HOPLITE_HTA_STRING
                || key->kind == HOPLITE_HTA_KEYWORD)
            && key->as.text.len == length
            && ngx_memcmp(key->as.text.data, name, length) == 0)
        {
            return map->as.map.items[i].value;
        }
    }
    return NULL;
}
