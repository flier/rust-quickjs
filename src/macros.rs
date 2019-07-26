#[doc(hidden)]
#[macro_export]
macro_rules! impl_foreign_type {
    ($type:ident, $reftype:ident) => {
        impl_foreign_type!(__impl_debug $type);
        impl_foreign_type!(__impl_debug $reftype);

        impl_foreign_type!(__impl_partial_eq $type, $type);
        impl_foreign_type!(__impl_partial_eq $type, $reftype);
        impl_foreign_type!(__impl_partial_eq $reftype, $reftype);
        impl_foreign_type!(__impl_partial_eq $reftype, $type);
    };

    (__impl_debug $name:ident) => {
        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(&self.as_ptr())
                    .finish()
            }
        }
    };

    (__impl_partial_eq $name:ident, $rhs:ident) => {
        impl ::std::cmp::PartialEq<$rhs> for $name {
            fn eq(&self, other: &$rhs) -> bool {
                self.as_ptr() == other.as_ptr()
            }
        }
    };
}
