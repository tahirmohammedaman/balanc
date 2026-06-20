//! A `Code` attribute builder (JVMS §4.7.3): emits raw bytecode and tracks
//! `max_stack`/`max_locals` automatically as each instruction is appended.
//!
//! Every method `codegen.rs` emits is straight-line — no `if`/`goto`, no loops, no
//! exception handlers (see `super::mod` for why: the one place a generated module
//! class would need data-dependent control flow, it instead calls into
//! `balanc.runtime.Ledger`, which is ordinary `javac`-compiled Java). A single running
//! stack depth is therefore exact, not just a conservative bound — there is no
//! branch target where two different incoming depths could disagree, so this builder
//! never needs to emit a `StackMapTable`.

use super::pool::ConstantPool;

pub const T_BOOLEAN: u8 = 4;
pub const T_INT: u8 = 10;
pub const T_LONG: u8 = 11;

pub struct CodeBuilder {
    code: Vec<u8>,
    stack: i32,
    max_stack: u16,
    max_locals: u16,
}

impl Default for CodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeBuilder {
    pub fn new() -> Self {
        CodeBuilder { code: Vec::new(), stack: 0, max_stack: 0, max_locals: 1 }
    }

    fn push(&mut self, words: i32) {
        self.stack += words;
        assert!(self.stack >= 0, "internal error: jvm codegen stack underflow");
        if self.stack as u16 > self.max_stack {
            self.max_stack = self.stack as u16;
        }
    }

    fn pop(&mut self, words: i32) {
        self.push(-words);
    }

    fn touch_local(&mut self, slot: u16, width: u16) {
        self.max_locals = self.max_locals.max(slot + width);
    }

    fn u8_(&mut self, b: u8) {
        self.code.push(b);
    }

    fn u16_(&mut self, v: u16) {
        self.code.extend_from_slice(&v.to_be_bytes());
    }

    // --- constants ---

    pub fn push_int(&mut self, v: i32, pool: &mut ConstantPool) {
        match v {
            -1 => self.u8_(0x02),                      // iconst_m1
            0 => self.u8_(0x03),                        // iconst_0
            1 => self.u8_(0x04),                        // iconst_1
            2 => self.u8_(0x05),                        // iconst_2
            3 => self.u8_(0x06),                        // iconst_3
            4 => self.u8_(0x07),                        // iconst_4
            5 => self.u8_(0x08),                        // iconst_5
            -128..=127 => {
                self.u8_(0x10); // bipush
                self.u8_(v as i8 as u8);
            }
            -32768..=32767 => {
                self.u8_(0x11); // sipush
                self.code.extend_from_slice(&(v as i16).to_be_bytes());
            }
            _ => {
                let idx = pool.integer(v);
                self.ldc(idx);
            }
        }
        self.push(1);
    }

    pub fn push_long(&mut self, v: i64, pool: &mut ConstantPool) {
        match v {
            0 => self.u8_(0x09), // lconst_0
            1 => self.u8_(0x0a), // lconst_1
            _ => {
                let idx = pool.long(v);
                self.u8_(0x14); // ldc2_w
                self.u16_(idx);
            }
        }
        self.push(2);
    }

    pub fn push_string(&mut self, s: &str, pool: &mut ConstantPool) {
        let idx = pool.string(s);
        self.ldc(idx);
        self.push(1);
    }

    fn ldc(&mut self, idx: u16) {
        if idx <= 0xff {
            self.u8_(0x12); // ldc
            self.u8_(idx as u8);
        } else {
            self.u8_(0x13); // ldc_w
            self.u16_(idx);
        }
    }

    // --- locals ---

    pub fn aload_this(&mut self) {
        self.u8_(0x2a); // aload_0
        self.push(1);
        self.touch_local(0, 1);
    }

    pub fn aload(&mut self, slot: u16) {
        match slot {
            0 => self.u8_(0x2a),
            1 => self.u8_(0x2b),
            2 => self.u8_(0x2c),
            3 => self.u8_(0x2d),
            _ => {
                self.u8_(0x19); // aload
                self.u8_(slot as u8);
            }
        }
        self.push(1);
        self.touch_local(slot, 1);
    }

    pub fn astore(&mut self, slot: u16) {
        match slot {
            0 => self.u8_(0x4b),
            1 => self.u8_(0x4c),
            2 => self.u8_(0x4d),
            3 => self.u8_(0x4e),
            _ => {
                self.u8_(0x3a); // astore
                self.u8_(slot as u8);
            }
        }
        self.pop(1);
        self.touch_local(slot, 1);
    }

    pub fn lload(&mut self, slot: u16) {
        match slot {
            0 => self.u8_(0x1e),
            1 => self.u8_(0x1f),
            2 => self.u8_(0x20),
            3 => self.u8_(0x21),
            _ => {
                self.u8_(0x16); // lload
                self.u8_(slot as u8);
            }
        }
        self.push(2);
        self.touch_local(slot, 2);
    }

    pub fn lstore(&mut self, slot: u16) {
        match slot {
            0 => self.u8_(0x3f),
            1 => self.u8_(0x40),
            2 => self.u8_(0x41),
            3 => self.u8_(0x42),
            _ => {
                self.u8_(0x37); // lstore
                self.u8_(slot as u8);
            }
        }
        self.pop(2);
        self.touch_local(slot, 2);
    }

    // --- arrays ---

    pub fn newarray_primitive(&mut self, atype: u8) {
        self.u8_(0xbc); // newarray
        self.u8_(atype);
        // pops length(1), pushes arrayref(1): net 0
    }

    pub fn anewarray(&mut self, class_internal: &str, pool: &mut ConstantPool) {
        let idx = pool.class(class_internal);
        self.u8_(0xbd); // anewarray
        self.u16_(idx);
        // pops length(1), pushes arrayref(1): net 0
    }

    pub fn dup(&mut self) {
        self.u8_(0x59);
        self.push(1);
    }

    pub fn aastore(&mut self) {
        self.u8_(0x53);
        self.pop(3);
    }

    pub fn iastore(&mut self) {
        self.u8_(0x4f);
        self.pop(3);
    }

    pub fn bastore(&mut self) {
        self.u8_(0x54);
        self.pop(3);
    }

    pub fn laload(&mut self) {
        self.u8_(0x2f);
        self.pop(2); // arrayref(1) + index(1)
        self.push(2); // long
    }

    // --- fields ---

    pub fn getfield(&mut self, class_internal: &str, name: &str, descriptor: &str, pool: &mut ConstantPool) {
        let idx = pool.fieldref(class_internal, name, descriptor);
        self.u8_(0xb4); // getfield
        self.u16_(idx);
        self.pop(1); // objectref
        self.push(descriptor_words(descriptor) as i32);
    }

    pub fn putfield(&mut self, class_internal: &str, name: &str, descriptor: &str, pool: &mut ConstantPool) {
        let idx = pool.fieldref(class_internal, name, descriptor);
        self.u8_(0xb5); // putfield
        self.u16_(idx);
        self.pop(1 + descriptor_words(descriptor) as i32); // objectref + value
    }

    pub fn getstatic(&mut self, class_internal: &str, name: &str, descriptor: &str, pool: &mut ConstantPool) {
        let idx = pool.fieldref(class_internal, name, descriptor);
        self.u8_(0xb2); // getstatic
        self.u16_(idx);
        self.push(descriptor_words(descriptor) as i32);
    }

    pub fn putstatic(&mut self, class_internal: &str, name: &str, descriptor: &str, pool: &mut ConstantPool) {
        let idx = pool.fieldref(class_internal, name, descriptor);
        self.u8_(0xb3); // putstatic
        self.u16_(idx);
        self.pop(descriptor_words(descriptor) as i32);
    }

    // --- calls ---

    pub fn new_(&mut self, class_internal: &str, pool: &mut ConstantPool) {
        let idx = pool.class(class_internal);
        self.u8_(0xbb); // new
        self.u16_(idx);
        self.push(1);
    }

    pub fn invokespecial(&mut self, class_internal: &str, name: &str, descriptor: &str, pool: &mut ConstantPool) {
        let idx = pool.methodref(class_internal, name, descriptor);
        self.u8_(0xb7);
        self.u16_(idx);
        let (args, ret) = call_words(descriptor);
        self.pop(1 + args as i32); // objectref + args
        self.push(ret as i32);
    }

    pub fn invokevirtual(&mut self, class_internal: &str, name: &str, descriptor: &str, pool: &mut ConstantPool) {
        let idx = pool.methodref(class_internal, name, descriptor);
        self.u8_(0xb6);
        self.u16_(idx);
        let (args, ret) = call_words(descriptor);
        self.pop(1 + args as i32);
        self.push(ret as i32);
    }

    pub fn invokestatic(&mut self, class_internal: &str, name: &str, descriptor: &str, pool: &mut ConstantPool) {
        let idx = pool.methodref(class_internal, name, descriptor);
        self.u8_(0xb8);
        self.u16_(idx);
        let (args, ret) = call_words(descriptor);
        self.pop(args as i32);
        self.push(ret as i32);
    }

    // --- arithmetic ---

    pub fn ladd(&mut self) {
        self.u8_(0x61);
        self.pop(2);
    }

    pub fn lsub(&mut self) {
        self.u8_(0x65);
        self.pop(2);
    }

    pub fn lmul(&mut self) {
        self.u8_(0x69);
        self.pop(2);
    }

    pub fn ldiv(&mut self) {
        self.u8_(0x6d);
        self.pop(2);
    }

    pub fn lrem(&mut self) {
        self.u8_(0x71);
        self.pop(2);
    }

    // --- returns ---

    pub fn return_void(&mut self) {
        self.u8_(0xb1);
    }

    pub fn areturn(&mut self) {
        self.u8_(0xb0);
        self.pop(1);
    }

    /// Finishes the method, returning `(max_stack, max_locals, code_bytes)` for a
    /// `Code` attribute — no exception table, no attributes on the `Code` attribute
    /// itself (line numbers/locals tables are out of scope, PLAN.md Slice 7).
    pub fn finish(self) -> (u16, u16, Vec<u8>) {
        (self.max_stack, self.max_locals, self.code)
    }
}

/// Word count (1 or 2, per JVMS §4.3.2/§4.10.1.5) of a single field descriptor —
/// `J`/`D` are two-word, everything else (`I`/`Z`/`L...;`/`[...`) is one word.
fn descriptor_words(field_descriptor: &str) -> u16 {
    match field_descriptor.as_bytes()[0] {
        b'J' | b'D' => 2,
        _ => 1,
    }
}

/// Parses a method descriptor `(ARGS)RET` into `(arg_words, return_words)`.
fn call_words(descriptor: &str) -> (u16, u16) {
    let bytes = descriptor.as_bytes();
    assert_eq!(bytes[0], b'(', "malformed descriptor: {descriptor}");
    let close = descriptor.find(')').expect("malformed descriptor: no ')'");
    let args = &bytes[1..close];
    let mut words = 0u16;
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            b'J' | b'D' => {
                words += 2;
                i += 1;
            }
            b'L' => {
                words += 1;
                while args[i] != b';' {
                    i += 1;
                }
                i += 1;
            }
            b'[' => {
                // Array types contribute one word regardless of element type or
                // dimension; skip the `[`s then the element descriptor.
                while args[i] == b'[' {
                    i += 1;
                }
                if args[i] == b'L' {
                    while args[i] != b';' {
                        i += 1;
                    }
                    i += 1;
                } else {
                    i += 1;
                }
                words += 1;
            }
            _ => {
                words += 1;
                i += 1;
            }
        }
    }
    let ret = &descriptor[close + 1..];
    let ret_words = if ret == "V" { 0 } else { descriptor_words(ret) };
    (words, ret_words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_words_parses_mixed_descriptors() {
        assert_eq!(call_words("(IJ)V"), (3, 0));
        assert_eq!(call_words("(JJJ)[J"), (6, 1));
        assert_eq!(call_words("(JJ)J"), (4, 2));
        assert_eq!(call_words("()Ljava/lang/String;"), (0, 1));
        assert_eq!(
            call_words("([Ljava/lang/String;[I[I[Z[Ljava/lang/String;[I)Ljava/lang/String;"),
            (6, 1)
        );
        assert_eq!(call_words("(I)V"), (1, 0));
        assert_eq!(call_words("()V"), (0, 0));
    }
}
