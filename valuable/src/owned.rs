#![allow(missing_docs)]

use indexmap::IndexMap;
use std::error::Error as StdError;
use std::path::PathBuf;

use crate::{NamedValues, Valuable, Value, Visit};

#[derive(Debug)]
pub enum ValueOwned {
    Unit,
    Number(Number),
    Bool(bool),
    Char(char),
    String(String),
    Path(PathBuf),
    Error(Box<dyn StdError>),
    Sequence(Sequence),
    Mapping(Mapping),
    Enum(Box<EnumValue>),
}

#[derive(Debug)]
pub enum Number {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    ISIZE(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    USIZE(usize),
    F32(f32),
    F64(f64),
}

impl Number {
    pub fn as_f64(&self) -> f64 {
        match self {
            Number::I8(num) => *num as f64,
            Number::I16(num) => *num as f64,
            Number::I32(num) => *num as f64,
            Number::I64(num) => *num as f64,
            Number::I128(num) => *num as f64,
            Number::ISIZE(num) => *num as f64,
            Number::U8(num) => *num as f64,
            Number::U16(num) => *num as f64,
            Number::U32(num) => *num as f64,
            Number::U64(num) => *num as f64,
            Number::U128(num) => *num as f64,
            Number::USIZE(num) => *num as f64,
            Number::F32(num) => *num as f64,
            Number::F64(num) => *num,
        }
    }
}

pub type Sequence = Vec<ValueOwned>;

#[derive(Debug)]
pub struct Mapping {
    pub map: IndexMap<String, ValueOwned>,
}

#[derive(Debug)]
pub struct EnumValue {
    pub variant: String,
    pub value: ValueOwned,
}

#[derive(Debug)]
struct Visitor {
    value: ValueOwned,
}

impl Visit for Visitor {
    fn visit_value(&mut self, value: Value<'_>) {
        match value {
            Value::I8(num) => self.value = ValueOwned::Number(Number::I8(num)),
            Value::I16(num) => self.value = ValueOwned::Number(Number::I16(num)),
            Value::I32(num) => self.value = ValueOwned::Number(Number::I32(num)),
            Value::I64(num) => self.value = ValueOwned::Number(Number::I64(num)),
            Value::I128(num) => self.value = ValueOwned::Number(Number::I128(num)),
            Value::Isize(num) => self.value = ValueOwned::Number(Number::ISIZE(num)),

            Value::U8(num) => self.value = ValueOwned::Number(Number::U8(num)),
            Value::U16(num) => self.value = ValueOwned::Number(Number::U16(num)),
            Value::U32(num) => self.value = ValueOwned::Number(Number::U32(num)),
            Value::U64(num) => self.value = ValueOwned::Number(Number::U64(num)),
            Value::U128(num) => self.value = ValueOwned::Number(Number::U128(num)),
            Value::Usize(num) => self.value = ValueOwned::Number(Number::USIZE(num)),

            Value::F32(num) => self.value = ValueOwned::Number(Number::F32(num)),
            Value::F64(num) => self.value = ValueOwned::Number(Number::F64(num)),

            Value::Bool(val) => self.value = ValueOwned::Bool(val),
            Value::Char(ch) => self.value = ValueOwned::Char(ch),
            Value::String(val) => self.value = ValueOwned::String(val.to_owned()),
            Value::Path(p) => self.value = ValueOwned::Path(p.to_owned()),
            Value::Error(_) => todo!("dont know what to do with error"),

            Value::Unit => self.value = ValueOwned::Unit,

            Value::Listable(listable) => {
                let mut visit = VisitList { list: Vec::new() };
                listable.visit(&mut visit);
                self.value = ValueOwned::Sequence(visit.list);
            }

            Value::Tuplable(tuplable) => {
                let mut visit = VisitList { list: Vec::new() };
                tuplable.visit(&mut visit);
                self.value = ValueOwned::Sequence(visit.list);
            }

            Value::Mappable(mappable) => {
                let mut visit = VisitMap {
                    map: IndexMap::new(),
                };
                mappable.visit(&mut visit);
                self.value = ValueOwned::Mapping(Mapping { map: visit.map });
            }

            Value::Structable(structable) => {
                let mut visit = VisitMap {
                    map: IndexMap::new(),
                };
                structable.visit(&mut visit);
                self.value = ValueOwned::Mapping(Mapping { map: visit.map });
            }

            Value::Enumerable(enumerable) => {
                let variant = enumerable.variant().name().to_string();
                let mut visit = VisitEnum {
                    value: ValueOwned::Unit,
                };
                enumerable.visit(&mut visit);
                self.value = ValueOwned::Enum(Box::new(EnumValue {
                    variant,
                    value: visit.value,
                }));
            }
        }
    }
}

struct VisitList {
    list: Vec<ValueOwned>,
}

impl Visit for VisitList {
    fn visit_unnamed_fields(&mut self, values: &[Value<'_>]) {
        self.list.extend(values.into_iter().map(into_owned));
    }
    fn visit_value(&mut self, value: Value<'_>) {
        self.list.push(into_owned(&value));
    }
}

struct VisitMap {
    map: IndexMap<String, ValueOwned>,
}

impl Visit for VisitMap {
    fn visit_named_fields(&mut self, named_values: &NamedValues<'_>) {
        for (k, v) in named_values {
            self.map.insert(k.name().to_owned(), into_owned(v));
        }
    }
    fn visit_value(&mut self, _: Value<'_>) {
        unreachable!()
    }
}

struct VisitEnum {
    value: ValueOwned,
}

impl Visit for VisitEnum {
    fn visit_unnamed_fields(&mut self, values: &[Value<'_>]) {
        if !values.is_empty() {
            self.value = ValueOwned::Sequence(values.into_iter().map(into_owned).collect());
        }
    }

    fn visit_named_fields(&mut self, named_values: &NamedValues<'_>) {
        if !named_values.is_empty() {
            self.value = ValueOwned::Mapping(Mapping {
                map: named_values
                    .into_iter()
                    .map(|(k, v)| (k.name().to_owned(), into_owned(v)))
                    .collect(),
            });
        }
    }

    fn visit_value(&mut self, _: Value<'_>) {
        unreachable!()
    }
}

pub(crate) fn into_owned<'a>(v: &Value<'a>) -> ValueOwned {
    let mut visit = Visitor {
        value: ValueOwned::Unit,
    };
    v.visit(&mut visit);
    visit.value
}
