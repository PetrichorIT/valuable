use crate::{Fields, NamedField, NamedValues, Slice, StructDef, Structable, Value, Visit};

use core::fmt;
use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use core::num::Wrapping;
use core::time::Duration;

/// A type that can be converted to a [`Value`].
///
/// `Valuable` types are inspected by defining a [`Visit`] implementation and
/// using it when calling [`Valuable::visit`]. See [`Visit`] documentation for
/// more details.
///
/// The `Valuable` procedural macro makes implementing `Valuable` easy. Users
/// can add add [`#[derive(Valuable)]`][macro] to their types.
///
/// `Valuable` provides implementations for many Rust primitives and standard
/// library types.
///
/// Types implementing `Valuable` may also implement one of the more specific
/// traits: [`Structable`], [`Enumerable`], [`Listable`], and [`Mappable`]. These traits
/// should be implemented when the type is a nested container of other `Valuable` types.
///
/// [`Value`]: Value
/// [`Visit`]: Visit
/// [`Valuable::visit`]: Valuable::visit
/// [`Structable`]: crate::Structable
/// [`Enumerable`]: crate::Enumerable
/// [`Listable`]: crate::Listable
/// [`Mappable`]: crate::Mappable
/// [macro]: macro@crate::Valuable
pub trait Valuable {
    /// Converts self into a [`Value`] instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use valuable::Valuable;
    ///
    /// let _ = "hello".as_value();
    /// ```
    fn as_value(&self) -> Value<'_>;

    /// Calls the relevant method on [`Visit`] to extract data from `self`.
    ///
    /// This method is used to extract type-specific data from the value and is
    /// intended to be an implementation detail. For example, `Vec` implements
    /// `visit` by calling [`visit_value()`] on each of its elements. Structs
    /// implement `visit` by calling [`visit_named_fields()`] or
    /// [`visit_unnamed_fields()`].
    ///
    /// Usually, users will call the [`visit`] function instead.
    ///
    /// [`Visit`]: Visit
    /// [`visit`]: visit()
    /// [`visit_value()`]: Visit::visit_value()
    /// [`visit_named_fields()`]: Visit::visit_named_fields()
    /// [`visit_unnamed_fields()`]: Visit::visit_unnamed_fields()
    fn visit(&self, visit: &mut dyn Visit);

    /// Calls [`Visit::visit_primitive_slice()`] with `self`.
    ///
    /// This method is an implementation detail used to optimize visiting
    /// primitive slices.
    ///
    /// [`Visit::visit_primitive_slice()`]: Visit::visit_primitive_slice
    fn visit_slice(slice: &[Self], visit: &mut dyn Visit)
    where
        Self: Sized,
    {
        for item in slice {
            visit.visit_value(item.as_value());
        }
    }
}

macro_rules! deref {
    (
        $(
            $(#[$attrs:meta])*
            $ty:ty,
        )*
    ) => {
        $(
            $(#[$attrs])*
            impl<T: ?Sized + Valuable> Valuable for $ty {
                fn as_value(&self) -> Value<'_> {
                    T::as_value(&**self)
                }

                fn visit(&self, visit: &mut dyn Visit) {
                    T::visit(&**self, visit);
                }
            }
        )*
    };
}

deref! {
    &T,
    &mut T,
    #[cfg(feature = "alloc")]
    alloc::boxed::Box<T>,
    #[cfg(feature = "alloc")]
    alloc::rc::Rc<T>,
    #[cfg(not(valuable_no_atomic_cas))]
    #[cfg(feature = "alloc")]
    alloc::sync::Arc<T>,
}

macro_rules! valuable {
    (
        $(
            $variant:ident($ty:ty),
        )*
    ) => {
        $(
            impl Valuable for $ty {
                fn as_value(&self) -> Value<'_> {
                    Value::$variant(*self)
                }

                fn visit(&self, visit: &mut dyn Visit) {
                    visit.visit_value(self.as_value());
                }

                fn visit_slice(slice: &[Self], visit: &mut dyn Visit)
                where
                    Self: Sized,
                {
                    visit.visit_primitive_slice(Slice::$variant(slice));
                }
            }
        )*
    };
}

valuable! {
    Bool(bool),
    Char(char),
    F32(f32),
    F64(f64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
}

macro_rules! nonzero {
    (
        $(
            $variant:ident($ty:ident),
        )*
    ) => {
        $(
            impl Valuable for core::num::$ty {
                fn as_value(&self) -> Value<'_> {
                    Value::$variant(self.get())
                }

                fn visit(&self, visit: &mut dyn Visit) {
                    visit.visit_value(self.as_value());
                }
            }
        )*
    };
}

nonzero! {
    I8(NonZeroI8),
    I16(NonZeroI16),
    I32(NonZeroI32),
    I64(NonZeroI64),
    I128(NonZeroI128),
    Isize(NonZeroIsize),
    U8(NonZeroU8),
    U16(NonZeroU16),
    U32(NonZeroU32),
    U64(NonZeroU64),
    U128(NonZeroU128),
    Usize(NonZeroUsize),
}

#[cfg(not(valuable_no_atomic))]
macro_rules! atomic {
    (
        $(
            $(#[$attrs:meta])*
            $variant:ident($ty:ident),
        )*
    ) => {
        $(
            $(#[$attrs])*
            impl Valuable for core::sync::atomic::$ty {
                fn as_value(&self) -> Value<'_> {
                    // Use SeqCst to match Debug and serde which use SeqCst.
                    // https://github.com/rust-lang/rust/blob/1.52.1/library/core/src/sync/atomic.rs#L1361-L1366
                    // https://github.com/serde-rs/serde/issues/1496
                    Value::$variant(self.load(core::sync::atomic::Ordering::SeqCst))
                }

                fn visit(&self, visit: &mut dyn Visit) {
                    visit.visit_value(self.as_value());
                }
            }
        )*
    };
}

#[cfg(not(valuable_no_atomic))]
atomic! {
    Bool(AtomicBool),
    I8(AtomicI8),
    I16(AtomicI16),
    I32(AtomicI32),
    #[cfg(not(valuable_no_atomic_64))]
    I64(AtomicI64),
    Isize(AtomicIsize),
    U8(AtomicU8),
    U16(AtomicU16),
    U32(AtomicU32),
    #[cfg(not(valuable_no_atomic_64))]
    U64(AtomicU64),
    Usize(AtomicUsize),
}

impl<T: Valuable> Valuable for Wrapping<T> {
    fn as_value(&self) -> Value<'_> {
        self.0.as_value()
    }

    fn visit(&self, visit: &mut dyn Visit) {
        self.0.visit(visit);
    }
}

impl Valuable for () {
    fn as_value(&self) -> Value<'_> {
        Value::Tuplable(self)
    }

    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_unnamed_fields(&[]);
    }
}

impl<T: Valuable> Valuable for Option<T> {
    fn as_value(&self) -> Value<'_> {
        match self {
            Some(v) => v.as_value(),
            None => Value::Unit,
        }
    }

    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(self.as_value());
    }
}

impl Valuable for &'_ str {
    fn as_value(&self) -> Value<'_> {
        Value::String(self)
    }

    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(Value::String(self));
    }

    fn visit_slice(slice: &[Self], visit: &mut dyn Visit)
    where
        Self: Sized,
    {
        visit.visit_primitive_slice(Slice::Str(slice));
    }
}

#[cfg(feature = "alloc")]
impl Valuable for alloc::string::String {
    fn as_value(&self) -> Value<'_> {
        Value::String(&self[..])
    }

    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(Value::String(self));
    }

    fn visit_slice(slice: &[Self], visit: &mut dyn Visit)
    where
        Self: Sized,
    {
        visit.visit_primitive_slice(Slice::String(slice));
    }
}

#[cfg(feature = "std")]
impl Valuable for &std::path::Path {
    fn as_value(&self) -> Value<'_> {
        Value::Path(self)
    }

    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(Value::Path(self));
    }
}

#[cfg(feature = "std")]
impl Valuable for std::path::PathBuf {
    fn as_value(&self) -> Value<'_> {
        Value::Path(self)
    }

    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(Value::Path(self));
    }
}

#[cfg(feature = "std")]
impl Valuable for dyn std::error::Error + 'static {
    fn as_value(&self) -> Value<'_> {
        Value::Error(self)
    }

    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(self.as_value());
    }
}

impl fmt::Debug for dyn Valuable + '_ {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.as_value();
        value.fmt(fmt)
    }
}

impl Valuable for IpAddr {
    fn as_value(&self) -> Value<'_> {
        match self {
            IpAddr::V4(addr) => addr.as_value(),
            IpAddr::V6(addr) => addr.as_value(),
        }
    }
    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(self.as_value());
    }
}

impl Structable for Ipv4Addr {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("Ipv4Addr", Fields::Unnamed(1))
    }
}

impl Valuable for Ipv4Addr {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }
    fn visit(&self, v: &mut dyn Visit) {
        v.visit_unnamed_fields(&[self.to_string().as_value()]);
    }
}

impl Structable for Ipv6Addr {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("Ipv6Addr", Fields::Unnamed(1))
    }
}

impl Valuable for Ipv6Addr {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }
    fn visit(&self, v: &mut dyn Visit) {
        v.visit_unnamed_fields(&[self.to_string().as_value()]);
    }
}

impl Valuable for SocketAddr {
    fn as_value(&self) -> Value<'_> {
        match self {
            SocketAddr::V4(addr) => addr.as_value(),
            SocketAddr::V6(addr) => addr.as_value(),
        }
    }
    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(self.as_value());
    }
}

static SOCKET_ADDR_FIELDS: &[NamedField<'static>] =
    &[NamedField::new("addr"), NamedField::new("port")];

impl Structable for SocketAddrV4 {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("SocketAddrV4", Fields::Named(SOCKET_ADDR_FIELDS))
    }
}

impl Valuable for SocketAddrV4 {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, v: &mut dyn Visit) {
        v.visit_named_fields(&NamedValues::new(
            SOCKET_ADDR_FIELDS,
            &[self.ip().as_value(), Value::U16(self.port())],
        ));
    }
}

impl Structable for SocketAddrV6 {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("SocketAddrV6", Fields::Named(SOCKET_ADDR_FIELDS))
    }
}

impl Valuable for SocketAddrV6 {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, v: &mut dyn Visit) {
        v.visit_named_fields(&NamedValues::new(
            SOCKET_ADDR_FIELDS,
            &[self.ip().as_value(), Value::U16(self.port())],
        ));
    }
}

// std::time

static DURATION_FIELDS: &[NamedField<'static>] =
    &[NamedField::new("secs"), NamedField::new("nanos")];

impl Structable for Duration {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("Duration", Fields::Named(DURATION_FIELDS))
    }
}

impl Valuable for Duration {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }
    fn visit(&self, v: &mut dyn Visit) {
        v.visit_named_fields(&NamedValues::new(
            SOCKET_ADDR_FIELDS,
            &[Value::U64(self.as_secs()), Value::U32(self.subsec_nanos())],
        ));
    }
}
