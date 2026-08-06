#[path = "../../../src/core.rs"]
mod core;
#[path = "../../../src/hta.rs"]
mod hta;
#[path = "../../../src/kernel.rs"]
mod kernel;
#[path = "../../../src/lang.rs"]
mod lang;
#[path = "../../../src/task.rs"]
mod task;

use core::Value;
use std::cell::RefCell;
use std::collections::VecDeque;

thread_local! {
    static EVENTS: RefCell<VecDeque<Vec<u8>>> = RefCell::new(VecDeque::new());
    static NEXT_TASK: RefCell<u64> = const { RefCell::new(1) };
}

#[no_mangle]
pub extern "C" fn hta_abi_version() -> i32 { 1 }

#[no_mangle]
pub extern "C" fn hta_alloc(size: usize) -> *mut u8 {
    unsafe { std::alloc::alloc(std::alloc::Layout::from_size_align(size.max(1), 1).unwrap()) }
}

#[no_mangle]
pub extern "C" fn hta_dealloc(pointer: *mut u8, size: usize) {
    if !pointer.is_null() {
        unsafe { std::alloc::dealloc(pointer, std::alloc::Layout::from_size_align(size.max(1), 1).unwrap()) }
    }
}

fn request(bytes: &[u8]) -> Result<Vec<u8>, String> {
    match hta::decode(bytes)? {
        Value::Vector(values) if values.len() == 2 => {
            if !matches!(&values[0], Value::String(target) if target == "digest") {
                return Err("hta/target-unknown".into());
            }
            match &values[1] {
                Value::Vector(arguments) if arguments.len() == 1 => match &arguments[0] {
                    Value::Bytes(bytes) => Ok(bytes.clone()),
                    Value::ByteBuffer(bytes) => Ok(bytes.borrow().clone()),
                    _ => Err("crypto.hash.sha256/digest expects bytes".into()),
                },
                _ => Err("crypto.hash.sha256/digest expects one argument".into()),
            }
        }
        _ => Err("hta/start expects [target arguments]".into()),
    }
}

fn error(message: String) -> Value {
    Value::Map(vec![
        (Value::Keyword("code".into()), Value::Keyword("crypto/digest-failed".into())),
        (Value::Keyword("message".into()), Value::String(message)),
        (Value::Keyword("origin".into()), Value::Keyword("wasm".into())),
        (Value::Keyword("retryable".into()), Value::Bool(false)),
    ].into_iter().collect())
}

#[no_mangle]
pub extern "C" fn hta_start(pointer: *const u8, size: usize) -> i64 {
    let bytes = if pointer.is_null() { &[][..] } else { unsafe { std::slice::from_raw_parts(pointer, size) } };
    let task = NEXT_TASK.with(|next| { let task=*next.borrow(); *next.borrow_mut()+=1; task });
    let (kind, value) = match request(bytes) { Ok(bytes) => (0, Value::Bytes(sha256(&bytes).to_vec())), Err(message) => (1, error(message)) };
    let event = Value::Vector(vec![Value::Number(kind), Value::Number(task as i64), value].into());
    if let Ok(frame) = hta::encode(&event) { EVENTS.with(|events| events.borrow_mut().push_back(frame)); }
    task as i64
}

fn output(bytes: Vec<u8>) -> i64 {
    let size=bytes.len(); let pointer=hta_alloc(size); if pointer.is_null(){return 0;}
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(),pointer,size); }
    (((pointer as u64)<<32)|(size as u64)) as i64
}

#[no_mangle]
pub extern "C" fn hta_next_event() -> i64 { EVENTS.with(|events| events.borrow_mut().pop_front().map(output).unwrap_or(0)) }
#[no_mangle]
pub extern "C" fn hta_poll() -> i32 { EVENTS.with(|events| events.borrow().len() as i32) }
#[no_mangle]
pub extern "C" fn hta_deliver(_pointer: *const u8, _size: usize) -> i32 { 1 }
#[no_mangle]
pub extern "C" fn hta_cancel(_task: i64) -> i32 { 1 }
#[no_mangle]
pub extern "C" fn hta_drop_task(_task: i64) -> i32 { 0 }

fn sha256(input: &[u8]) -> [u8; 32] {
    const INITIAL: [u32;8]=[0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
    const K:[u32;64]=[0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2];
    let bit_len=(input.len() as u64)*8; let mut message=input.to_vec(); message.push(0x80); while message.len()%64!=56{message.push(0);} message.extend_from_slice(&bit_len.to_be_bytes());
    let mut state=INITIAL;
    for chunk in message.chunks_exact(64){let mut w=[0u32;64];for(i,word)in chunk.chunks_exact(4).enumerate(){w[i]=u32::from_be_bytes(word.try_into().unwrap());}for i in 16..64{let s0=w[i-15].rotate_right(7)^w[i-15].rotate_right(18)^(w[i-15]>>3);let s1=w[i-2].rotate_right(17)^w[i-2].rotate_right(19)^(w[i-2]>>10);w[i]=w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);}let(mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut h)=(state[0],state[1],state[2],state[3],state[4],state[5],state[6],state[7]);for i in 0..64{let s1=e.rotate_right(6)^e.rotate_right(11)^e.rotate_right(25);let ch=(e&f)^(!e&g);let t1=h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);let s0=a.rotate_right(2)^a.rotate_right(13)^a.rotate_right(22);let maj=(a&b)^(a&c)^(b&c);let t2=s0.wrapping_add(maj);h=g;g=f;f=e;e=d.wrapping_add(t1);d=c;c=b;b=a;a=t1.wrapping_add(t2);}for(i,value)in[a,b,c,d,e,f,g,h].iter().enumerate(){state[i]=state[i].wrapping_add(*value);}}
    let mut output=[0u8;32];for(i,value)in state.iter().enumerate(){output[i*4..i*4+4].copy_from_slice(&value.to_be_bytes());}output
}

#[cfg(test)]
mod tests { use super::*; #[test] fn abc_vector(){assert_eq!(sha256(b"abc"),[0xba,0x78,0x16,0xbf,0x8f,0x01,0xcf,0xea,0x41,0x41,0x40,0xde,0x5d,0xae,0x22,0x23,0xb0,0x03,0x61,0xa3,0x96,0x17,0x7a,0x9c,0xb4,0x10,0xff,0x61,0xf2,0x00,0x15,0xad]);} }

#[no_mangle]
pub extern "C" fn hta_release(_pointer: *const u8, _size: usize) -> i32 { 0 }
