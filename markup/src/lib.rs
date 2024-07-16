pub use markup_proc_macro::{define, new};

mod escape;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("An io error occured while rendering: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Custom error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync + 'static>)
}

impl RenderError {
    pub fn wrap<E: std::error::Error + Send + Sync + 'static>(e: E) -> Self {
        Self::Other(Box::new(e))
    }
}

pub trait Render {
    fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError>;
}

pub trait RenderAttributeValue: Render {
    #[inline]
    fn is_none(&self) -> bool {
        false
    }

    #[inline]
    fn is_true(&self) -> bool {
        false
    }

    #[inline]
    fn is_false(&self) -> bool {
        false
    }
}

impl<T: Render> Render for Box<T> {
    #[inline]
    fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
        T::render(*self, writer)
    }
}

impl<T: RenderAttributeValue> RenderAttributeValue for Box<T> {
    #[inline]
    fn is_none(&self) -> bool {
        T::is_none(self)
    }

    #[inline]
    fn is_true(&self) -> bool {
        T::is_true(self)
    }

    #[inline]
    fn is_false(&self) -> bool {
        T::is_false(self)
    }
}

impl Render for bool {
    #[inline]
    fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
        if self {
            writer.write_all(b"true")?;
        } else {
            writer.write_all(b"false")?;
        }
        Ok(())
    }
}

impl RenderAttributeValue for bool {
    #[inline]
    fn is_true(&self) -> bool {
        *self
    }

    #[inline]
    fn is_false(&self) -> bool {
        !self
    }
}

impl<T: Render> Render for Option<T> {
    #[inline]
    fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
        match self {
            Some(t) => t.render(writer),
            None => Ok(()),
        }
    }
}

impl<T: RenderAttributeValue> RenderAttributeValue for Option<T> {
    #[inline]
    fn is_none(&self) -> bool {
        self.is_none()
    }
}

#[cfg(feature="raw_disp")]
mod _raw_disp {
    use super::{Render, RenderAttributeValue};
    
    pub struct Raw<T: std::fmt::Display>(pub T);

    impl<T: std::fmt::Display> Render for Raw<T> {
        #[inline]
        fn render(self, writer: &mut impl std::io::Write) -> Result<(), super::RenderError> {
            write!(writer, "{}", self.0)?;
            Ok(())
        }
    }

    impl<T: std::fmt::Display> RenderAttributeValue for Raw<T> {}
}

#[cfg(feature="raw_disp")]
#[inline(always)]
pub fn raw_disp(value: impl std::fmt::Display) -> impl Render + RenderAttributeValue {
    _raw_disp::Raw(value)
}

pub struct RawBytes<T: AsRef<[u8]>>(T);

impl<T: AsRef<[u8]>> Render for RawBytes<T> {
    #[inline(always)]
    fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
        writer.write_all(self.0.as_ref())?;
        Ok(())
    }
}

impl<T: AsRef<[u8]>> RenderAttributeValue for RawBytes<T> {}

#[inline(always)]
pub fn raw_bytes<T: AsRef<[u8]>>(raw: T) -> impl Render {
    RawBytes(raw)
}

macro_rules! tfor {
    (for $ty:ident in [$($typ:ident),*] $tt:tt) => {
        $( const _: () = { type $ty = $typ; tfor! { @extract $tt } }; )*
    };
    (@extract { $($tt:tt)* }) => { $($tt)* };
}


impl Render for char {
    #[inline(always)]
    fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
        let mut b = [0u8;4];
        let s = self.encode_utf8(&mut b);
        writer.write_all(s.as_bytes())?;
        Ok(())
    }
}

impl RenderAttributeValue for char {}

tfor! {
    for Ty in [f32, f64] {
        impl Render for Ty {
            #[inline]
            fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
                write!(writer, "{}", self)?;
                Ok(())
            }
        }

        impl RenderAttributeValue for Ty {
        }
    }
}

tfor! {
    for Ty in [u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize] {
        impl Render for Ty {
            #[inline]
            fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
                #[cfg(feature = "itoa")] {
                    let mut buffer = itoa::Buffer::new();
                    let str = buffer.format(self);
                    writer.write(str.as_bytes())?;
                    Ok(())
                }
                #[cfg(not(feature = "itoa"))] {
                    write!(writer, "{}", self)?;
                    Ok(())
                }
            }
        }

        impl RenderAttributeValue for Ty {
        }
    }
}

impl Render for &str {
    #[inline]
    fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
        escape::escape(self.as_bytes(), writer)?;
        Ok(())
    }
}

impl RenderAttributeValue for &str {}

impl Render for String {
    #[inline]
    fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
        self.as_str().render(writer)
    }
}

impl RenderAttributeValue for String {}

impl<'a> Render for std::borrow::Cow<'a, str> {
    #[inline]
    fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
        (&*self).render(writer)
    }
}

impl<'a> RenderAttributeValue for std::borrow::Cow<'a, str> {}

macro_rules! tuple_impl {
    ($($ident:ident)+) => {
        impl<$($ident: Render,)+> Render for ($($ident,)+) {
            #[allow(non_snake_case)]
            #[inline]
            fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
                let ($($ident,)+) = self;
                $($ident.render(writer)?;)+
                Ok(())
            }
        }

        impl<$($ident: RenderAttributeValue,)+> RenderAttributeValue for ($($ident,)+) {
        }
    }
}

tuple_impl! { A }
tuple_impl! { A B }
tuple_impl! { A B C }
tuple_impl! { A B C D }
tuple_impl! { A B C D E }
tuple_impl! { A B C D E F }
tuple_impl! { A B C D E F G }
tuple_impl! { A B C D E F G H }
tuple_impl! { A B C D E F G H I }
tuple_impl! { A B C D E F G H I J }

pub struct DynRender<'a> {
    f: Box<dyn Fn(&mut dyn std::io::Write) -> Result<(), RenderError> + 'a>,
}

pub fn new<'a, F>(f: F) -> DynRender<'a>
where
    F: Fn(&mut dyn std::io::Write) -> Result<(), RenderError> + 'a,
{
    DynRender { f: Box::new(f) }
}

impl<'a> Render for DynRender<'a> {
    #[inline]
    fn render(self, writer: &mut impl std::io::Write) -> Result<(), RenderError> {
        (self.f)(writer)
    }
}

#[inline]
pub fn doctype() -> impl Render {
    raw_bytes(b"<!DOCTYPE html>")
}

