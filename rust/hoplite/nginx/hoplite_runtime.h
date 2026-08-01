#ifndef HOPLITE_RUNTIME_H
#define HOPLITE_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

typedef struct hoplite_runtime hoplite_runtime_t;

typedef struct {
    uint8_t *data;
    size_t len;
} hoplite_buffer_t;

uint32_t hoplite_abi_version(void);
hoplite_runtime_t *hoplite_runtime_new(void);
void hoplite_runtime_free(hoplite_runtime_t *runtime);

uint64_t hoplite_work_start(hoplite_runtime_t *runtime,
                            const uint8_t *function,
                            size_t function_len,
                            const uint8_t *input,
                            size_t input_len);

size_t hoplite_work_poll(hoplite_runtime_t *runtime);
int hoplite_work_next_event(hoplite_runtime_t *runtime, hoplite_buffer_t *output);
void hoplite_buffer_free(uint8_t *data, size_t len);

int hoplite_work_send(hoplite_runtime_t *runtime,
                      uint64_t work,
                      const uint8_t *message,
                      size_t message_len);

int hoplite_call_resolve(hoplite_runtime_t *runtime,
                         uint64_t call,
                         const uint8_t *payload,
                         size_t payload_len);

int hoplite_call_reject(hoplite_runtime_t *runtime,
                        uint64_t call,
                        const uint8_t *payload,
                        size_t payload_len);

int hoplite_work_cancel(hoplite_runtime_t *runtime, uint64_t work);
int hoplite_work_close(hoplite_runtime_t *runtime, uint64_t work);

#endif
