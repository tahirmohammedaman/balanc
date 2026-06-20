//! The JVM constant pool (JVMS §4.4), built incrementally: each `utf8`/`class`/
//! `fieldref`/etc. call either returns the index of an already-interned entry or
//! appends a new one and returns its fresh index. Entries are written to `bytes` at
//! insertion time (the pool's on-disk order is exactly insertion order, so there's
//! nothing to defer) — `finish` just prepends the JVMS-mandated `constant_pool_count`
//! (one greater than the number of *slots* used, not the number of entries: a `Long`
//! occupies two slots and burns the index after it, per JVMS §4.4.5).

use std::collections::HashMap;

const TAG_UTF8: u8 = 1;
const TAG_INTEGER: u8 = 3;
const TAG_LONG: u8 = 5;
const TAG_CLASS: u8 = 7;
const TAG_STRING: u8 = 8;
const TAG_FIELDREF: u8 = 9;
const TAG_METHODREF: u8 = 10;
const TAG_NAME_AND_TYPE: u8 = 12;

#[derive(Default)]
pub struct ConstantPool {
    bytes: Vec<u8>,
    next_index: u16,
    utf8: HashMap<String, u16>,
    class: HashMap<String, u16>,
    name_and_type: HashMap<(u16, u16), u16>,
    fieldref: HashMap<(u16, u16), u16>,
    methodref: HashMap<(u16, u16), u16>,
    string: HashMap<u16, u16>,
    integer: HashMap<i32, u16>,
    long: HashMap<i64, u16>,
}

impl ConstantPool {
    pub fn new() -> Self {
        ConstantPool { next_index: 1, ..Default::default() }
    }

    pub fn utf8(&mut self, s: &str) -> u16 {
        if let Some(&idx) = self.utf8.get(s) {
            return idx;
        }
        let idx = self.alloc(1);
        self.bytes.push(TAG_UTF8);
        let raw = s.as_bytes();
        self.bytes.extend_from_slice(&(raw.len() as u16).to_be_bytes());
        self.bytes.extend_from_slice(raw);
        self.utf8.insert(s.to_string(), idx);
        idx
    }

    pub fn class(&mut self, internal_name: &str) -> u16 {
        if let Some(&idx) = self.class.get(internal_name) {
            return idx;
        }
        let name_idx = self.utf8(internal_name);
        let idx = self.alloc(1);
        self.bytes.push(TAG_CLASS);
        self.bytes.extend_from_slice(&name_idx.to_be_bytes());
        self.class.insert(internal_name.to_string(), idx);
        idx
    }

    pub fn name_and_type(&mut self, name: &str, descriptor: &str) -> u16 {
        let name_idx = self.utf8(name);
        let desc_idx = self.utf8(descriptor);
        if let Some(&idx) = self.name_and_type.get(&(name_idx, desc_idx)) {
            return idx;
        }
        let idx = self.alloc(1);
        self.bytes.push(TAG_NAME_AND_TYPE);
        self.bytes.extend_from_slice(&name_idx.to_be_bytes());
        self.bytes.extend_from_slice(&desc_idx.to_be_bytes());
        self.name_and_type.insert((name_idx, desc_idx), idx);
        idx
    }

    pub fn fieldref(&mut self, class_internal: &str, name: &str, descriptor: &str) -> u16 {
        let class_idx = self.class(class_internal);
        let nat_idx = self.name_and_type(name, descriptor);
        if let Some(&idx) = self.fieldref.get(&(class_idx, nat_idx)) {
            return idx;
        }
        let idx = self.alloc(1);
        self.bytes.push(TAG_FIELDREF);
        self.bytes.extend_from_slice(&class_idx.to_be_bytes());
        self.bytes.extend_from_slice(&nat_idx.to_be_bytes());
        self.fieldref.insert((class_idx, nat_idx), idx);
        idx
    }

    pub fn methodref(&mut self, class_internal: &str, name: &str, descriptor: &str) -> u16 {
        let class_idx = self.class(class_internal);
        let nat_idx = self.name_and_type(name, descriptor);
        if let Some(&idx) = self.methodref.get(&(class_idx, nat_idx)) {
            return idx;
        }
        let idx = self.alloc(1);
        self.bytes.push(TAG_METHODREF);
        self.bytes.extend_from_slice(&class_idx.to_be_bytes());
        self.bytes.extend_from_slice(&nat_idx.to_be_bytes());
        self.methodref.insert((class_idx, nat_idx), idx);
        idx
    }

    pub fn string(&mut self, s: &str) -> u16 {
        let utf8_idx = self.utf8(s);
        if let Some(&idx) = self.string.get(&utf8_idx) {
            return idx;
        }
        let idx = self.alloc(1);
        self.bytes.push(TAG_STRING);
        self.bytes.extend_from_slice(&utf8_idx.to_be_bytes());
        self.string.insert(utf8_idx, idx);
        idx
    }

    pub fn integer(&mut self, v: i32) -> u16 {
        if let Some(&idx) = self.integer.get(&v) {
            return idx;
        }
        let idx = self.alloc(1);
        self.bytes.push(TAG_INTEGER);
        self.bytes.extend_from_slice(&v.to_be_bytes());
        self.integer.insert(v, idx);
        idx
    }

    /// A `Long` entry occupies indices `idx` *and* `idx + 1` (JVMS §4.4.5) — the next
    /// entry allocated after this one skips index `idx + 1` entirely.
    pub fn long(&mut self, v: i64) -> u16 {
        if let Some(&idx) = self.long.get(&v) {
            return idx;
        }
        let idx = self.alloc(2);
        self.bytes.push(TAG_LONG);
        self.bytes.extend_from_slice(&v.to_be_bytes());
        self.long.insert(v, idx);
        idx
    }

    fn alloc(&mut self, slots: u16) -> u16 {
        let idx = self.next_index;
        self.next_index += slots;
        idx
    }

    /// Writes `constant_pool_count` followed by every entry, in insertion order.
    pub fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.next_index.to_be_bytes());
        out.extend_from_slice(&self.bytes);
    }
}
