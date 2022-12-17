#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

struct JetstreamEncodeResult {
  uintptr_t len;
  const uint8_t *data;
};

struct JetstreamDatasetWithQuality {
  uint64_t t;
  int32_t *i32s;
  uint32_t *q;
};

struct JetstreamDecodedIndexResult {
  bool ok;
  uint64_t t;
  int32_t value;
  uint32_t q;
};

extern "C" {

void jetstream_new_encoder(const uint8_t *id,
                           uintptr_t i32_count,
                           uintptr_t sampling_rate,
                           uintptr_t samples_per_message);

void jetstream_new_decoder(const uint8_t *id,
                           uintptr_t i32_count,
                           uintptr_t sampling_rate,
                           uintptr_t samples_per_message);

void jetstream_remove_encoder(const uint8_t *id);

void jetstream_remove_decoder(const uint8_t *id);

/// Encodes a single sample of data. If this completes a message, the encoded message data is returned.
JetstreamEncodeResult jetstream_encode(const uint8_t *id,
                                       uint64_t t,
                                       const int32_t *i32s,
                                       const uint32_t *q);

/// Performs batch encoding of an entire message. The encoded message data is returned.
JetstreamEncodeResult jetstream_encode_all(const uint8_t *id,
                                           const JetstreamDatasetWithQuality *data,
                                           uintptr_t length);

/// Performs Slipstream decoding from raw byte data. Results are stored in the struct,
/// and `jetstream_get_decoded()` or `jetstream_get_decoded_index()` should be used to
/// access results from C.
bool jetstream_decode(const uint8_t *id, const uint8_t *data, uintptr_t length);

/// Returns a single data point (with timestamp and quality). This is very inefficient because it
/// needs to be called repeatedly for each encoded variable and time-step.
JetstreamDecodedIndexResult jetstream_get_decoded_index(const uint8_t *id,
                                                        uintptr_t sample_index,
                                                        uintptr_t value_index);

/// Maps decoded Slipstream data into a slice of `JetstreamDatasetWithQuality`, which is allocated
/// in C code.
/// This provides an efficient way of copying all decoded data from a message from Go to C.
bool jetstream_get_decoded(const uint8_t *id, JetstreamDatasetWithQuality *data, uintptr_t length);

} // extern "C"
