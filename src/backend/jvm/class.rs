//! Assembles a `.class` file (JVMS §4.1) from a `ConstantPool` plus a list of fields
//! and methods. Targets major version 61 (Java 17), not 65 (Java 21) — D-042: the
//! widest net that costs nothing balanc's runtime shape actually needs, since JVM 21
//! (the confirmed toolchain, see PLAN.md OQ-8) still loads and verifies an
//! older-version class with no special flag.

use super::code::CodeBuilder;
use super::pool::ConstantPool;

const MAJOR_VERSION: u16 = 61;

pub const ACC_PUBLIC: u16 = 0x0001;
pub const ACC_PRIVATE: u16 = 0x0002;
pub const ACC_STATIC: u16 = 0x0008;
pub const ACC_FINAL: u16 = 0x0010;
pub const ACC_SUPER: u16 = 0x0020;

struct Field {
    access: u16,
    name_index: u16,
    descriptor_index: u16,
}

struct Method {
    access: u16,
    name_index: u16,
    descriptor_index: u16,
    max_stack: u16,
    max_locals: u16,
    code: Vec<u8>,
}

pub struct ClassFile {
    pub pool: ConstantPool,
    this_class: u16,
    super_class: u16,
    fields: Vec<Field>,
    methods: Vec<Method>,
    code_attr_name: u16,
}

impl ClassFile {
    /// `this_internal`/`super_internal` are internal (slash-separated) class names,
    /// e.g. `"Coffee"` or `"balanc/runtime/Ledger"`.
    pub fn new(this_internal: &str, super_internal: &str) -> Self {
        let mut pool = ConstantPool::new();
        let this_class = pool.class(this_internal);
        let super_class = pool.class(super_internal);
        let code_attr_name = pool.utf8("Code");
        ClassFile { pool, this_class, super_class, fields: Vec::new(), methods: Vec::new(), code_attr_name }
    }

    pub fn add_field(&mut self, access: u16, name: &str, descriptor: &str) {
        let name_index = self.pool.utf8(name);
        let descriptor_index = self.pool.utf8(descriptor);
        self.fields.push(Field { access, name_index, descriptor_index });
    }

    pub fn add_method(&mut self, access: u16, name: &str, descriptor: &str, body: CodeBuilder) {
        let name_index = self.pool.utf8(name);
        let descriptor_index = self.pool.utf8(descriptor);
        let (max_stack, max_locals, code) = body.finish();
        self.methods.push(Method { access, name_index, descriptor_index, max_stack, max_locals, code });
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0xCAFEBABEu32.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes()); // minor_version
        out.extend_from_slice(&MAJOR_VERSION.to_be_bytes());
        self.pool.write(&mut out);
        out.extend_from_slice(&(ACC_PUBLIC | ACC_FINAL | ACC_SUPER).to_be_bytes());
        out.extend_from_slice(&self.this_class.to_be_bytes());
        out.extend_from_slice(&self.super_class.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes()); // interfaces_count

        out.extend_from_slice(&(self.fields.len() as u16).to_be_bytes());
        for f in &self.fields {
            out.extend_from_slice(&f.access.to_be_bytes());
            out.extend_from_slice(&f.name_index.to_be_bytes());
            out.extend_from_slice(&f.descriptor_index.to_be_bytes());
            out.extend_from_slice(&0u16.to_be_bytes()); // attributes_count
        }

        out.extend_from_slice(&(self.methods.len() as u16).to_be_bytes());
        for m in &self.methods {
            out.extend_from_slice(&m.access.to_be_bytes());
            out.extend_from_slice(&m.name_index.to_be_bytes());
            out.extend_from_slice(&m.descriptor_index.to_be_bytes());
            out.extend_from_slice(&1u16.to_be_bytes()); // attributes_count (Code only)

            // Code attribute (JVMS 4.7.3): no exception table, no sub-attributes —
            // line-number/local-variable tables are out of scope (PLAN.md Slice 7).
            let mut code_attr = Vec::new();
            code_attr.extend_from_slice(&m.max_stack.to_be_bytes());
            code_attr.extend_from_slice(&m.max_locals.to_be_bytes());
            code_attr.extend_from_slice(&(m.code.len() as u32).to_be_bytes());
            code_attr.extend_from_slice(&m.code);
            code_attr.extend_from_slice(&0u16.to_be_bytes()); // exception_table_length
            code_attr.extend_from_slice(&0u16.to_be_bytes()); // attributes_count

            out.extend_from_slice(&self.code_attr_name.to_be_bytes());
            out.extend_from_slice(&(code_attr.len() as u32).to_be_bytes());
            out.extend_from_slice(&code_attr);
        }

        out.extend_from_slice(&0u16.to_be_bytes()); // attributes_count (class-level)
        out
    }
}
