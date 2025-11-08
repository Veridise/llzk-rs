//! Helper macros

/// Creates a newtype wrapper around the given type.
///
/// Automatically generates an implementation of [`From`] for working with the newtype.
macro_rules! newtype {
    ($wrapped:ty, $wrapper:ident) => {
        #[doc = concat!("Newtype wrapper over [`", stringify!($wrapped),"`]")]
        pub struct $wrapper($wrapped);

        impl From<$wrapped> for $wrapper {
            fn from(value: $wrapped) -> Self {
                Self(value)
            }
        }

        impl $crate::Wrapped for $wrapped {
            type Wrapper = $wrapper;

            fn wrap(self) -> Self::Wrapper {
                self.into()
            }
        }
    };
    ($wrapped:ty, $wrapper:ident with $( $traits:ident),+ $(,)?) => {
        $crate::macros::newtype!($wrapped, $wrapper);
        $crate::macros::__newtype_impls!( $wrapper with $( $traits, )*);
    };
}

macro_rules! __newtype_impls {
    ($wrapper:ident with) => {
    };
    ($wrapper:ident with $trait:ident $( ,$traits:ident )* $(,)?) => {
        $crate::macros::__newtype_impls!( $wrapper impl $trait);
        $crate::macros::__newtype_impls!( $wrapper with $( $traits, )*);
    };
    ($wrapper:ident impl Copy) => {
                impl Copy for $wrapper {}

    };
    ($wrapper:ident impl Clone) => {
        impl Clone for $wrapper {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }
    };
    ( $wrapper:ident impl Debug) => {
        impl std::fmt::Debug for $wrapper {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Debug::fmt(&self.0, f)
            }
        }
    };
}

pub(crate) use __newtype_impls;
pub(crate) use newtype;
