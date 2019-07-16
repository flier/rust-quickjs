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
        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(&self.as_ptr())
                    .finish()
            }
        }
    };

    (__impl_partial_eq $name:ident, $rhs:ident) => {
        impl ::core::cmp::PartialEq<$rhs> for $name {
            fn eq(&self, other: &$rhs) -> bool {
                self.as_ptr() == other.as_ptr()
            }
        }
    };
}
