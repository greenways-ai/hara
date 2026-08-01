#include "openhal_hta.h"

#define OH_NIL 0
#define OH_FALSE 1
#define OH_TRUE 2
#define OH_I64 3
#define OH_STRING 4
#define OH_BYTES 5
#define OH_KEYWORD 6
#define OH_VECTOR 9
#define OH_MAP 11

static const u_char openhal_magic[] = {'H', 'T', 'A', '1'};

typedef struct {
    ngx_pool_t *pool;
    const u_char *data;
    size_t len;
    size_t cursor;
} openhal_reader_t;

typedef struct {
    u_char *data;
    size_t len;
    size_t cursor;
} openhal_writer_t;

static ngx_int_t openhal_read_value(openhal_reader_t *reader,
                                    openhal_hta_value_t **output);

static ngx_int_t
openhal_take(openhal_reader_t *reader, size_t size, const u_char **output)
{
    if (size > reader->len || reader->cursor > reader->len - size) {
        return NGX_ERROR;
    }
    *output = reader->data + reader->cursor;
    reader->cursor += size;
    return NGX_OK;
}

static ngx_int_t
openhal_read_u32(openhal_reader_t *reader, uint32_t *output)
{
    const u_char *data;
    if (openhal_take(reader, 4, &data) != NGX_OK) {
        return NGX_ERROR;
    }
    *output = ((uint32_t) data[0] << 24)
            | ((uint32_t) data[1] << 16)
            | ((uint32_t) data[2] << 8)
            | (uint32_t) data[3];
    return NGX_OK;
}

static ngx_int_t
openhal_read_text(openhal_reader_t *reader, ngx_str_t *output)
{
    uint32_t length;
    const u_char *data;
    if (openhal_read_u32(reader, &length) != NGX_OK
        || openhal_take(reader, length, &data) != NGX_OK)
    {
        return NGX_ERROR;
    }
    output->data = (u_char *) data;
    output->len = length;
    return NGX_OK;
}

static openhal_hta_value_t *
openhal_new_value(openhal_reader_t *reader, openhal_hta_kind_t kind)
{
    openhal_hta_value_t *value = ngx_pcalloc(reader->pool, sizeof(*value));
    if (value != NULL) {
        value->kind = kind;
    }
    return value;
}

static ngx_int_t
openhal_read_sequence(openhal_reader_t *reader, openhal_hta_value_t *value)
{
    uint32_t count;
    size_t i;

    if (openhal_read_u32(reader, &count) != NGX_OK) {
        return NGX_ERROR;
    }
    if (count > 100000) {
        return NGX_ERROR;
    }

    value->as.vector.count = count;
    value->as.vector.items = ngx_pcalloc(
        reader->pool, sizeof(openhal_hta_value_t *) * count);
    if (count != 0 && value->as.vector.items == NULL) {
        return NGX_ERROR;
    }

    for (i = 0; i < count; i++) {
        if (openhal_read_value(reader, &value->as.vector.items[i]) != NGX_OK) {
            return NGX_ERROR;
        }
    }
    return NGX_OK;
}

static ngx_int_t
openhal_read_map(openhal_reader_t *reader, openhal_hta_value_t *value)
{
    uint32_t count;
    size_t i;

    if (openhal_read_u32(reader, &count) != NGX_OK) {
        return NGX_ERROR;
    }
    if (count > 100000) {
        return NGX_ERROR;
    }

    value->as.map.count = count;
    value->as.map.items = ngx_pcalloc(
        reader->pool, sizeof(openhal_hta_pair_t) * count);
    if (count != 0 && value->as.map.items == NULL) {
        return NGX_ERROR;
    }

    for (i = 0; i < count; i++) {
        if (openhal_read_value(reader, &value->as.map.items[i].key) != NGX_OK
            || openhal_read_value(reader, &value->as.map.items[i].value) != NGX_OK)
        {
            return NGX_ERROR;
        }
    }
    return NGX_OK;
}

static ngx_int_t
openhal_read_value(openhal_reader_t *reader, openhal_hta_value_t **output)
{
    const u_char *tag_data;
    const u_char *number;
    openhal_hta_value_t *value;
    uint64_t raw;
    ngx_uint_t tag;

    if (openhal_take(reader, 1, &tag_data) != NGX_OK) {
        return NGX_ERROR;
    }
    tag = tag_data[0];

    switch (tag) {
    case OH_NIL:
        value = openhal_new_value(reader, OPENHAL_HTA_NIL);
        break;
    case OH_FALSE:
    case OH_TRUE:
        value = openhal_new_value(reader, OPENHAL_HTA_BOOL);
        if (value != NULL) {
            value->as.boolean = tag == OH_TRUE;
        }
        break;
    case OH_I64:
        value = openhal_new_value(reader, OPENHAL_HTA_I64);
        if (value == NULL || openhal_take(reader, 8, &number) != NGX_OK) {
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
        value = openhal_new_value(
            reader,
            tag == OH_STRING ? OPENHAL_HTA_STRING
            : tag == OH_BYTES ? OPENHAL_HTA_BYTES
                              : OPENHAL_HTA_KEYWORD);
        if (value == NULL || openhal_read_text(reader, &value->as.text) != NGX_OK) {
            return NGX_ERROR;
        }
        break;
    case OH_VECTOR:
        value = openhal_new_value(reader, OPENHAL_HTA_VECTOR);
        if (value == NULL || openhal_read_sequence(reader, value) != NGX_OK) {
            return NGX_ERROR;
        }
        break;
    case OH_MAP:
        value = openhal_new_value(reader, OPENHAL_HTA_MAP);
        if (value == NULL || openhal_read_map(reader, value) != NGX_OK) {
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
openhal_hta_decode(ngx_pool_t *pool, const u_char *data, size_t len,
                   openhal_hta_value_t **value)
{
    openhal_reader_t reader;

    if (len < sizeof(openhal_magic)
        || ngx_memcmp(data, openhal_magic, sizeof(openhal_magic)) != 0)
    {
        return NGX_ERROR;
    }

    reader.pool = pool;
    reader.data = data;
    reader.len = len;
    reader.cursor = sizeof(openhal_magic);

    if (openhal_read_value(&reader, value) != NGX_OK || reader.cursor != len) {
        return NGX_ERROR;
    }
    return NGX_OK;
}

static ngx_int_t
openhal_write(openhal_writer_t *writer, const void *data, size_t len)
{
    if (len > writer->len || writer->cursor > writer->len - len) {
        return NGX_ERROR;
    }
    ngx_memcpy(writer->data + writer->cursor, data, len);
    writer->cursor += len;
    return NGX_OK;
}

static ngx_int_t
openhal_write_byte(openhal_writer_t *writer, u_char value)
{
    return openhal_write(writer, &value, 1);
}

static ngx_int_t
openhal_write_u32(openhal_writer_t *writer, uint32_t value)
{
    u_char bytes[4];
    bytes[0] = (u_char) (value >> 24);
    bytes[1] = (u_char) (value >> 16);
    bytes[2] = (u_char) (value >> 8);
    bytes[3] = (u_char) value;
    return openhal_write(writer, bytes, sizeof(bytes));
}

static ngx_int_t
openhal_write_text(openhal_writer_t *writer, u_char tag, const ngx_str_t *value)
{
    if (value->len > UINT32_MAX
        || openhal_write_byte(writer, tag) != NGX_OK
        || openhal_write_u32(writer, (uint32_t) value->len) != NGX_OK)
    {
        return NGX_ERROR;
    }
    return openhal_write(writer, value->data, value->len);
}

static ngx_int_t
openhal_write_i64(openhal_writer_t *writer, int64_t value)
{
    uint64_t raw = (uint64_t) value;
    u_char bytes[8];
    bytes[0] = (u_char) (raw >> 56);
    bytes[1] = (u_char) (raw >> 48);
    bytes[2] = (u_char) (raw >> 40);
    bytes[3] = (u_char) (raw >> 32);
    bytes[4] = (u_char) (raw >> 24);
    bytes[5] = (u_char) (raw >> 16);
    bytes[6] = (u_char) (raw >> 8);
    bytes[7] = (u_char) raw;
    if (openhal_write_byte(writer, OH_I64) != NGX_OK) {
        return NGX_ERROR;
    }
    return openhal_write(writer, bytes, sizeof(bytes));
}

static ngx_int_t
openhal_write_pair(openhal_writer_t *writer, const char *key,
                   const ngx_str_t *value)
{
    ngx_str_t name;
    name.data = (u_char *) key;
    name.len = ngx_strlen(key);
    return openhal_write_text(writer, OH_KEYWORD, &name) == NGX_OK
        && openhal_write_text(writer, OH_STRING, value) == NGX_OK
        ? NGX_OK : NGX_ERROR;
}

static size_t
openhal_request_capacity(ngx_http_request_t *request)
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
openhal_hta_encode_request(ngx_http_request_t *request, ngx_str_t *output)
{
    openhal_writer_t writer;
    ngx_list_part_t *part;
    ngx_table_elt_t *header;
    ngx_uint_t i, header_count = 0;
    ngx_str_t method, uri, path, args, remote;
    size_t capacity = openhal_request_capacity(request);

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

    if (openhal_write(&writer, openhal_magic, sizeof(openhal_magic)) != NGX_OK
        || openhal_write_byte(&writer, OH_MAP) != NGX_OK
        || openhal_write_u32(&writer, 6) != NGX_OK
        || openhal_write_pair(&writer, "method", &method) != NGX_OK
        || openhal_write_pair(&writer, "uri", &uri) != NGX_OK
        || openhal_write_pair(&writer, "path", &path) != NGX_OK
        || openhal_write_pair(&writer, "query-string", &args) != NGX_OK
        || openhal_write_pair(&writer, "remote-address", &remote) != NGX_OK)
    {
        return NGX_ERROR;
    }

    {
        ngx_str_t headers_key = ngx_string("headers");
        if (openhal_write_text(&writer, OH_KEYWORD, &headers_key) != NGX_OK
            || openhal_write_byte(&writer, OH_MAP) != NGX_OK
            || openhal_write_u32(&writer, (uint32_t) header_count) != NGX_OK)
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
        if (openhal_write_text(&writer, OH_STRING, &header[i].key) != NGX_OK
            || openhal_write_text(&writer, OH_STRING, &header[i].value) != NGX_OK)
        {
            return NGX_ERROR;
        }
    }

    output->len = writer.cursor;
    return NGX_OK;
}

ngx_int_t
openhal_hta_encode_string(ngx_pool_t *pool, const ngx_str_t *value,
                          ngx_str_t *output)
{
    openhal_writer_t writer;
    size_t capacity = sizeof(openhal_magic) + 1 + 4 + value->len;

    output->data = ngx_pnalloc(pool, capacity);
    if (output->data == NULL) {
        return NGX_ERROR;
    }
    writer.data = output->data;
    writer.len = capacity;
    writer.cursor = 0;

    if (openhal_write(&writer, openhal_magic, sizeof(openhal_magic)) != NGX_OK
        || openhal_write_text(&writer, OH_STRING, value) != NGX_OK)
    {
        return NGX_ERROR;
    }
    output->len = writer.cursor;
    return NGX_OK;
}

ngx_int_t
openhal_hta_text(const openhal_hta_value_t *value, ngx_str_t *output)
{
    if (value == NULL
        || (value->kind != OPENHAL_HTA_STRING
            && value->kind != OPENHAL_HTA_BYTES
            && value->kind != OPENHAL_HTA_KEYWORD))
    {
        return NGX_ERROR;
    }
    *output = value->as.text;
    return NGX_OK;
}

ngx_int_t
openhal_hta_number(const openhal_hta_value_t *value, int64_t *output)
{
    if (value == NULL || value->kind != OPENHAL_HTA_I64) {
        return NGX_ERROR;
    }
    *output = value->as.i64;
    return NGX_OK;
}

openhal_hta_value_t *
openhal_hta_map_get(const openhal_hta_value_t *map, const char *name)
{
    size_t length = ngx_strlen(name);
    size_t i;
    openhal_hta_value_t *key;

    if (map == NULL || map->kind != OPENHAL_HTA_MAP) {
        return NULL;
    }

    for (i = 0; i < map->as.map.count; i++) {
        key = map->as.map.items[i].key;
        if (key != NULL
            && (key->kind == OPENHAL_HTA_STRING
                || key->kind == OPENHAL_HTA_KEYWORD)
            && key->as.text.len == length
            && ngx_memcmp(key->as.text.data, name, length) == 0)
        {
            return map->as.map.items[i].value;
        }
    }
    return NULL;
}
