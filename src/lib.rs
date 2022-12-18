// Copyright (c) 2021 Synaptec Ltd
// Copyright (c) 2022 Richard Lincoln
//
// This program is free software: you can redistribute it and/or
// modify it under the terms of the GNU Affero General Public License
// as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
// See the GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with this program.
// If not, see <https://www.gnu.org/licenses/>.
use jetstream::{Decoder, Encoder};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::sync::{Mutex, MutexGuard};
use std::{ptr, slice};
use uuid::{Bytes, Uuid};

lazy_static! {
    static ref ENC_LIST: Mutex<HashMap<Uuid, Encoder>> = Mutex::new(HashMap::new());
    static ref DEC_LIST: Mutex<HashMap<Uuid, Decoder>> = Mutex::new(HashMap::new());
}

#[no_mangle]
pub extern "C" fn jetstream_new_encoder(
    id: *const u8,
    i32_count: usize,
    sampling_rate: usize,
    samples_per_message: usize,
) {
    let id_slice = unsafe { slice::from_raw_parts(id, 16) };
    let id_bytes: Bytes = id_slice.try_into().unwrap();
    let uuid = Uuid::from_bytes(id_bytes);

    let enc = Encoder::new(uuid, i32_count, sampling_rate, samples_per_message);

    let mut list: MutexGuard<HashMap<Uuid, Encoder>> = ENC_LIST.lock().unwrap();
    list.insert(uuid, enc);
}

#[no_mangle]
pub extern "C" fn jetstream_new_decoder(
    id: *const u8,
    i32_count: usize,
    sampling_rate: usize,
    samples_per_message: usize,
) {
    let id_slice = unsafe { slice::from_raw_parts(id, 16) };
    let id_bytes: Bytes = id_slice.try_into().unwrap();
    let uuid = Uuid::from_bytes(id_bytes);

    let dec = Decoder::new(uuid, i32_count, sampling_rate, samples_per_message);

    let mut list: MutexGuard<HashMap<Uuid, Decoder>> = DEC_LIST.lock().unwrap();
    // list.push_back(dec);
    list.insert(uuid, dec);
}

#[no_mangle]
pub extern "C" fn jetstream_remove_encoder(id: *const u8) {
    let id_slice = unsafe { slice::from_raw_parts(id, 16) };
    let id_bytes: Bytes = id_slice.try_into().unwrap();
    let uuid = Uuid::from_bytes(id_bytes);

    ENC_LIST.lock().unwrap().remove(&uuid);
}

#[no_mangle]
pub extern "C" fn jetstream_remove_decoder(id: *const u8) {
    let id_slice = unsafe { slice::from_raw_parts(id, 16) };
    let id_bytes: Bytes = id_slice.try_into().unwrap();
    let uuid = Uuid::from_bytes(id_bytes);

    DEC_LIST.lock().unwrap().remove(&uuid);
}

#[repr(C)]
pub struct JetstreamEncodeResult {
    len: usize,
    data: *const u8,
}

/// Encodes a single sample of data. If this completes a message, the encoded message data is returned.
#[no_mangle]
pub extern "C" fn jetstream_encode(
    id: *const u8,
    t: u64,
    i32s: *const i32,
    q: *const u32,
) -> JetstreamEncodeResult {
    let id_slice = unsafe { slice::from_raw_parts(id, 16) };
    let id_bytes: Bytes = id_slice.try_into().unwrap();
    let uuid = Uuid::from_bytes(id_bytes);

    let mut list: MutexGuard<HashMap<Uuid, Encoder>> = ENC_LIST.lock().unwrap();
    let enc_opt: Option<&mut Encoder> = list.get_mut(&uuid);
    if enc_opt.is_none() {
        println!("id not found: {}", uuid);
        return JetstreamEncodeResult {
            len: 0,
            data: ptr::null(),
        };
    }
    let enc = enc_opt.unwrap();

    // let i32s_slice = unsafe { Vec::from_raw_parts(i32s, enc.i32_count, enc.i32_count) };
    // let q_slice = unsafe { Vec::from_raw_parts(q, enc.i32_count, enc.i32_count) };
    let i32s_slice = unsafe { slice::from_raw_parts(i32s, enc.i32_count) };
    let q_slice = unsafe { slice::from_raw_parts(q, enc.i32_count) };

    let mut data = jetstream::DatasetWithQuality {
        t,
        i32s: i32s_slice.to_vec(), // TODO: avoid copy
        q: q_slice.to_vec(),
    };

    // encode this data sample
    match enc.encode(&mut data) {
        Err(err) => {
            println!("encode error: {}", err);
            JetstreamEncodeResult {
                len: 0,
                data: ptr::null(),
            }
        }
        Ok((buf, len)) => {
            // need to use ManuallyDrop to copy bytes to C, data must be free'd later
            let buf = ManuallyDrop::new(buf);
            JetstreamEncodeResult {
                len,
                data: buf.as_ptr(),
            }
        }
    }
}

#[repr(C)]
pub struct JetstreamDatasetWithQuality {
    t: u64,
    i32s: *mut i32,
    q: *mut u32,
}

/// Performs batch encoding of an entire message. The encoded message data is returned.
#[no_mangle]
pub extern "C" fn jetstream_encode_all(
    id: *const u8,
    data: *const JetstreamDatasetWithQuality,
    length: usize,
) -> JetstreamEncodeResult {
    let id_slice = unsafe { slice::from_raw_parts(id, 16) };
    let id_bytes: Bytes = id_slice.try_into().unwrap();
    let uuid = Uuid::from_bytes(id_bytes);

    let mut list: MutexGuard<HashMap<Uuid, Encoder>> = ENC_LIST.lock().unwrap();
    let enc_opt: Option<&mut Encoder> = list.get_mut(&uuid);
    if enc_opt.is_none() {
        println!("id not found: {}", uuid);
        return JetstreamEncodeResult {
            len: 0,
            data: ptr::null(),
        };
    }
    let enc = enc_opt.unwrap();

    // convert array of DatasetWithQuality "owned" by C code into Go slice
    // datasetSlice := (*[1 << 30]C.struct_DatasetWithQuality)(unsafe.Pointer(data))[:length:length]
    // let dataset_slice = unsafe { Vec::from_raw_parts(data, length, length) };
    let dataset_slice = unsafe { slice::from_raw_parts(data, length) };

    for i in 0..dataset_slice.len() {
        // let s = dataset_slice.get(i).unwrap();
        // similar to above, convert C arrays into Go slices
        // Int32Slice := (*[1 << 30]int32)(unsafe.Pointer(datasetSlice[s].Int32s))[:enc.Int32Count:enc.Int32Count]
        // QSlice := (*[1 << 30]uint32)(unsafe.Pointer(datasetSlice[s].Q))[:enc.Int32Count:enc.Int32Count]

        // let i32s_slice =
        //     unsafe { Vec::from_raw_parts(dataset_slice[i].i32s, enc.i32_count, enc.i32_count) };
        // let q_slice =
        //     unsafe { Vec::from_raw_parts(dataset_slice[i].q, enc.i32_count, enc.i32_count) };
        let i32s_slice = unsafe { slice::from_raw_parts(dataset_slice[i].i32s, enc.i32_count) };
        let q_slice = unsafe { slice::from_raw_parts(dataset_slice[i].q, enc.i32_count) };

        let dataset = jetstream::DatasetWithQuality {
            t: dataset_slice[i].t as u64,
            i32s: i32s_slice.to_vec(), // TODO: avoid copy
            q: q_slice.to_vec(),
        };

        // encode this data sample
        match enc.encode(&dataset) {
            Err(err) => {
                println!("encode error: {}", err);
                return JetstreamEncodeResult {
                    len: 0,
                    data: ptr::null(),
                };
            }
            Ok((buf, len)) => {
                // need to use ManuallyDrop to copy bytes to C, data must be free'd later
                let buf = ManuallyDrop::new(buf);
                if len > 0 {
                    return JetstreamEncodeResult {
                        len,
                        data: buf.as_ptr(),
                    };
                }
            }
        }
    }

    JetstreamEncodeResult {
        len: 0,
        data: ptr::null(),
    }
}

/// Performs Slipstream decoding from raw byte data. Results are stored in the struct,
/// and `jetstream_get_decoded()` or `jetstream_get_decoded_index()` should be used to
/// access results from C.
#[no_mangle]
pub extern "C" fn jetstream_decode(id: *const u8, data: *const u8, length: usize) -> bool {
    let id_slice = unsafe { slice::from_raw_parts(id, 16) };
    let id_bytes: Bytes = id_slice.try_into().unwrap();
    let uuid = Uuid::from_bytes(id_bytes);

    // let dec = find_dec_by_id(uuid);
    let mut list: MutexGuard<HashMap<Uuid, Decoder>> = DEC_LIST.lock().unwrap();
    let dec_opt: Option<&mut Decoder> = list.get_mut(&uuid);
    if dec_opt.is_none() {
        println!("id not found: {}", uuid);
        return false;
    }
    let dec = dec_opt.unwrap();

    let data_slice = unsafe {
        // unsafe.Slice((*byte)(data), length)
        // Vec::from_raw_parts(data, length, length)
        slice::from_raw_parts(data, length)
    };

    // encode this data sample
    match dec.decode_to_buffer(&data_slice, length) {
        Err(_) => false,
        Ok(()) => true,
    }
}

#[repr(C)]
pub struct JetstreamDecodedIndexResult {
    ok: bool,
    t: u64,
    value: i32,
    q: u32,
}

/// Returns a single data point (with timestamp and quality). This is very inefficient because it
/// needs to be called repeatedly for each encoded variable and time-step.
#[no_mangle]
pub extern "C" fn jetstream_get_decoded_index(
    id: *const u8,
    sample_index: usize,
    value_index: usize,
) -> JetstreamDecodedIndexResult {
    let id_slice = unsafe { slice::from_raw_parts(id, 16) };
    let id_bytes: Bytes = id_slice.try_into().unwrap();
    let uuid = Uuid::from_bytes(id_bytes);

    // let dec = find_dec_by_id(uuid);
    let mut list: MutexGuard<HashMap<Uuid, Decoder>> = DEC_LIST.lock().unwrap();
    let dec_opt: Option<&mut Decoder> = list.get_mut(&uuid);
    if dec_opt.is_none() {
        println!("id not found: {}", uuid);
        return JetstreamDecodedIndexResult {
            ok: false,
            t: 0,
            value: 0,
            q: 0,
        };
    }
    let dec = dec_opt.unwrap();

    if sample_index >= dec.out.len() || value_index >= dec.out[sample_index].i32s.len() {
        return JetstreamDecodedIndexResult {
            ok: false,
            t: 0,
            value: 0,
            q: 0,
        };
    }

    JetstreamDecodedIndexResult {
        ok: true,
        t: dec.out[sample_index].t,
        value: dec.out[sample_index].i32s[value_index],
        q: dec.out[sample_index].q[value_index],
    }
}

/// Maps decoded Slipstream data into a slice of `JetstreamDatasetWithQuality`, which is allocated
/// in C code.
/// This provides an efficient way of copying all decoded data from a message from Go to C.
#[no_mangle]
pub extern "C" fn jetstream_get_decoded(
    id: *const u8,
    data: *mut JetstreamDatasetWithQuality,
    length: usize,
) -> bool {
    let id_slice = unsafe { slice::from_raw_parts(id, 16) };
    let id_bytes: Bytes = id_slice.try_into().unwrap();
    let uuid = Uuid::from_bytes(id_bytes);

    // let dec = find_dec_by_id(uuid);
    let mut list: MutexGuard<HashMap<Uuid, Decoder>> = DEC_LIST.lock().unwrap();
    let dec_opt: Option<&mut Decoder> = list.get_mut(&uuid);
    if dec_opt.is_none() {
        println!("id not found: {}", uuid);
        return false;
    }
    let dec = dec_opt.unwrap();

    // convert array of DatasetWithQuality "owned" by C code into Go slice
    // datasetSlice := (*[1 << 30]C.struct_DatasetWithQuality)(unsafe.Pointer(data))[:length:length]
    // let mut dataset_slice = unsafe { Vec::from_raw_parts(data, length, length) };
    let dataset_slice = unsafe { slice::from_raw_parts_mut(data, length) };

    for s in 0..dataset_slice.len() {
        dataset_slice[s].t = dec.out[s].t as u64;

        // similar to above, convert C arrays into Go slices
        // Int32Slice := (*[1 << 30]int32)(unsafe.Pointer(datasetSlice[s].Int32s))[:dec.Int32Count:dec.Int32Count]
        // QSlice := (*[1 << 30]uint32)(unsafe.Pointer(datasetSlice[s].Q))[:dec.Int32Count:dec.Int32Count]
        // let mut i32_slice =
        //     unsafe { Vec::from_raw_parts(dataset_slice[s].i32s, dec.i32_count, dec.i32_count) };
        // let mut q_slice =
        //     unsafe { Vec::from_raw_parts(dataset_slice[s].q, dec.i32_count, dec.i32_count) };
        let i32_slice = unsafe { slice::from_raw_parts_mut(dataset_slice[s].i32s, dec.i32_count) };
        let q_slice = unsafe { slice::from_raw_parts_mut(dataset_slice[s].q, dec.i32_count) };

        for i in 0..dec.i32_count {
            i32_slice[i] = dec.out[s].i32s[i];
            q_slice[i] = dec.out[s].q[i];
        }
    }
    true
}
