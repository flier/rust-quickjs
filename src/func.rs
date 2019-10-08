use failure::Error;
use foreign_types::ForeignTypeRef;

use crate::{ffi, undefined, value::FALSE, ContextRef, Local, NewAtom, NewValue, Value};

pub trait Args {
    type Values: AsRef<[ffi::JSValue]>;

    fn into_values(self, ctxt: &ContextRef) -> Self::Values;
}

impl<T> Args for T
where
    T: NewValue + Sized,
{
    type Values = [ffi::JSValue; 1];

    fn into_values(self, ctxt: &ContextRef) -> Self::Values {
        [self.new_value(ctxt)]
    }
}

impl<T> Args for &[T]
where
    T: NewValue + Clone,
{
    type Values = Vec<ffi::JSValue>;

    fn into_values(self, ctxt: &ContextRef) -> Self::Values {
        self.iter().map(|v| v.clone().new_value(ctxt)).collect()
    }
}

macro_rules! array_args {
    ($($N:expr)+) => {
        $(
            impl<T> Args for [T; $N]
            where
                T: NewValue,
            {
                type Values = Vec<ffi::JSValue>;

                fn into_values(self, ctxt: &ContextRef) -> Self::Values {
                    let len = self.len();
                    let mut data = std::mem::ManuallyDrop::new(self);

                    (0..len).map(|idx| unsafe {
                        std::ptr::read(data.get_unchecked_mut(idx)).new_value(ctxt)
                    }).collect()
                }
            }
        )*
    };
}

array_args! {
    0  1  2  3  4  5  6  7  8  9
    10 11 12 13 14 15 16 17 18 19
    20 21 22 23 24 25 26 27 28 29
    30 31 32
}

macro_rules! tuple_args {
    () => {
        impl Args for () {
            type Values = [ffi::JSValue; 0];

            fn into_values(self, _ctxt: &ContextRef) -> Self::Values {
                []
            }
        }
    };

    ($($name:ident)+) => {
        impl<$( $name ),*> Args for ($( $name, )*)
        where
            $( $name: NewValue, )*
        {
            type Values = [ffi::JSValue; count!($( $name )*)];

            #[allow(non_snake_case)]
            fn into_values(self, ctxt: &ContextRef) -> Self::Values {
                let ( $($name,)* ) = self;

                [ $( $name.new_value(ctxt), )* ]
            }
        }
    }
}

macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

tuple_args! {}
tuple_args! { A }
tuple_args! { A B }
tuple_args! { A B C }
tuple_args! { A B C D }
tuple_args! { A B C D E }
tuple_args! { A B C D E F }
tuple_args! { A B C D E F G }
tuple_args! { A B C D E F G H }
tuple_args! { A B C D E F G H I }
tuple_args! { A B C D E F G H I J }
tuple_args! { A B C D E F G H I J K }
tuple_args! { A B C D E F G H I J K L }
tuple_args! { A B C D E F G H I J K L M }
tuple_args! { A B C D E F G H I J K L M N }
tuple_args! { A B C D E F G H I J K L M N O }
tuple_args! { A B C D E F G H I J K L M N O P }
tuple_args! { A B C D E F G H I J K L M N O P Q }
tuple_args! { A B C D E F G H I J K L M N O P Q R }
tuple_args! { A B C D E F G H I J K L M N O P Q R S }
tuple_args! { A B C D E F G H I J K L M N O P Q R S T }

impl<'a> Local<'a, Value> {
    pub fn call<T: Args>(&self, this: Option<&Value>, args: T) -> Result<Local<Value>, Error> {
        self.ctxt.call(self, this, args)
    }

    pub fn invoke<N: NewAtom, T: Args>(&self, atom: N, args: T) -> Result<Local<Value>, Error> {
        self.ctxt.invoke(self, atom, args)
    }

    pub fn call_constructor<T: Args>(&self, args: T) -> Result<Local<Value>, Error> {
        self.ctxt.call_constructor(self, args)
    }

    pub fn call_constructor2<T: Args>(
        &self,
        new_target: Option<&Value>,
        args: T,
    ) -> Result<Local<Value>, Error> {
        self.ctxt.call_constructor2(self, new_target, args)
    }
}

impl ContextRef {
    pub fn is_function(&self, val: &Value) -> bool {
        unsafe { ffi::JS_IsFunction(self.as_ptr(), val.raw()) != FALSE }
    }

    pub fn is_constructor(&self, val: &Value) -> bool {
        unsafe { ffi::JS_IsConstructor(self.as_ptr(), val.raw()) != FALSE }
    }

    pub fn call<T: Args>(
        &self,
        func: &Value,
        this: Option<&Value>,
        args: T,
    ) -> Result<Local<Value>, Error> {
        let args = args.into_values(self);
        let args = args.as_ref();
        let ret = {
            unsafe {
                ffi::JS_Call(
                    self.as_ptr(),
                    func.raw(),
                    this.map_or_else(|| undefined().raw(), |v| v.raw()),
                    args.len() as i32,
                    args.as_ptr() as *mut _,
                )
            }
        };

        for arg in args {
            self.free_value(*arg);
        }

        self.bind(ret).ok()
    }

    pub fn invoke<N: NewAtom, T: Args>(
        &self,
        this: &Value,
        atom: N,
        args: T,
    ) -> Result<Local<Value>, Error> {
        let atom = atom.new_atom(self);
        let args = args.into_values(self);
        let args = args.as_ref();

        let res = self.bind(unsafe {
            ffi::JS_Invoke(
                self.as_ptr(),
                this.raw(),
                atom,
                args.len() as i32,
                args.as_ptr() as *mut _,
            )
        });
        self.free_atom(atom);
        for arg in args {
            self.free_value(*arg);
        }

        res.ok()
    }

    pub fn call_constructor<T: Args>(&self, func: &Value, args: T) -> Result<Local<Value>, Error> {
        let args = args.into_values(self);
        let args = args.as_ref();
        let ret = unsafe {
            ffi::JS_CallConstructor(
                self.as_ptr(),
                func.raw(),
                args.len() as i32,
                args.as_ptr() as *mut _,
            )
        };

        for arg in args {
            self.free_value(*arg);
        }

        self.bind(ret).ok()
    }

    pub fn call_constructor2<T: Args>(
        &self,
        func: &Value,
        new_target: Option<&Value>,
        args: T,
    ) -> Result<Local<Value>, Error> {
        let args = args.into_values(self);
        let args = args.as_ref();
        let ret = unsafe {
            ffi::JS_CallConstructor2(
                self.as_ptr(),
                func.raw(),
                new_target.map_or_else(|| undefined().raw(), |v| v.raw()),
                args.len() as i32,
                args.as_ptr() as *mut _,
            )
        };

        for arg in args {
            self.free_value(*arg);
        }

        self.bind(ret).ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, Eval, Runtime};

    #[test]
    fn call() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        ctxt.eval::<_, ()>(
            r#"
function fib(n)
{
    if (n <= 0)
        return 0;
    else if (n == 1)
        return 1;
    else
        return fib(n - 1) + fib(n - 2);
}

function Product(name, price) {
    this.name = name;
    this.price = price;
}
        "#,
            Eval::GLOBAL,
        )
        .unwrap();

        let global = ctxt.global_object();

        let fib = global.get_property("fib").unwrap();

        assert!(fib.is_function());

        assert_eq!(fib.call(None, [10]).unwrap().as_int().unwrap(), 55);

        let product_ctor = global.get_property("Product").unwrap();

        assert!(product_ctor.is_function());
        assert!(product_ctor.is_constructor());

        let product = product_ctor.call_constructor(("foobar", 30)).unwrap();

        assert_eq!(product.get_property("name").unwrap().to_string(), "foobar");
        assert_eq!(product.get_property("price").unwrap().as_int().unwrap(), 30);
    }
}
