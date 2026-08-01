#ifndef OPENHAL_RUNTIME_H
#define OPENHAL_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

typedef struct openhal_runtime openhal_runtime_t;

typedef struct {
    uint8_t *data;
    size_t len;
} openhal_buffer_t;

uint32_t openhal_abi_version(void);
openhal_runtime_t *openhal_runtime_new(void);
void openhal_runtime_free(openhal_runtime_t *runtime);

uint64_t openhal_start(openhal_runtime_t *runtime,
                       const uint8_t *source,
                       size_t source_len,
                       const uint8_t *binding,
                       size_t binding_len);

size_t openhal_poll(openhal_runtime_t *runtime);
int openhal_next_event(openhal_runtime_t *runtime, openhal_buffer_t *output);
void openhal_buffer_free(uint8_t *data, size_t len);

int openhal_deliver(openhal_runtime_t *runtime,
                    uint64_t call,
                    int success,
                    const uint8_t *payload,
                    size_t payload_len);

int openhal_cancel(openhal_runtime_t *runtime, uint64_t task);
int openhal_drop_task(openhal_runtime_t *runtime, uint64_t task);

#endif
