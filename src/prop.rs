use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ptr;
use std::slice;

use failure::{format_err, Error};
use foreign_types::ForeignTypeRef;

use crate::{
    ffi,
    value::{FALSE, TRUE},
    Atom, ContextRef, Local, NewAtom, NewValue, Value,
};

bitflags! {
    /// Flags for property
    pub struct Prop: u32 {
        /// This property descriptor may be changed or deleted from the corresponding object.
        const CONFIGURABLE = ffi::JS_PROP_CONFIGURABLE;
        /// The value associated with the property may be changed with an assignment operator.
        const WRITABLE = ffi::JS_PROP_WRITABLE;
        /// This property shows up during enumeration of the properties on the corresponding object.
        const ENUMERABLE = ffi::JS_PROP_ENUMERABLE;
        const C_W_E = ffi::JS_PROP_C_W_E;
        const PROP_LENGTH = ffi::JS_PROP_LENGTH;
        const TMASK = ffi::JS_PROP_TMASK;
        const NORMAL = ffi::JS_PROP_NORMAL;
        const GETSET = ffi::JS_PROP_GETSET;
        const VARREF = ffi::JS_PROP_VARREF;
        const AUTOINIT = ffi::JS_PROP_AUTOINIT;

        const HAS_SHIFT = ffi::JS_PROP_HAS_SHIFT;
        const HAS_CONFIGURABLE = ffi::JS_PROP_HAS_CONFIGURABLE;
        const HAS_WRITABLE = ffi::JS_PROP_HAS_WRITABLE;
        const HAS_ENUMERABLE = ffi::JS_PROP_HAS_ENUMERABLE;
        /// Has function which serves as a getter for the property.
        const HAS_GET = ffi::JS_PROP_HAS_GET;
        /// Has function which serves as a setter for the property.
        const HAS_SET = ffi::JS_PROP_HAS_SET;
        /// Has value associated with the property.
        const HAS_VALUE = ffi::JS_PROP_HAS_VALUE;

        const THROW = ffi::JS_PROP_THROW;
        const THROW_STRICT = ffi::JS_PROP_THROW_STRICT;

        const NO_ADD = ffi::JS_PROP_NO_ADD;
        const NO_EXOTIC = ffi::JS_PROP_NO_EXOTIC;
    }
}

bitflags! {
    /// Flags for `get_own_property_names`
    pub struct Names: u32 {
        const STRING = ffi::JS_GPN_STRING_MASK;
        const SYMBOL = ffi::JS_GPN_SYMBOL_MASK;
        /// only include the enumerable properties
        const ENUM_ONLY = ffi::JS_GPN_ENUM_ONLY;
    }
}

/// Get a property value on an object.
pub trait GetProperty {
    /// Get a property value on an object.
    fn get_property<'a>(&self, ctxt: &'a ContextRef, this: &Value) -> Option<Local<'a, Value>>;
}

impl GetProperty for &str {
    fn get_property<'a>(&self, ctxt: &'a ContextRef, this: &Value) -> Option<Local<'a, Value>> {
        ctxt.bind(unsafe {
            ffi::JS_GetPropertyStr(
                ctxt.as_ptr(),
                this.raw(),
                CString::new(*self).expect("prop").as_ptr(),
            )
        })
        .check_undefined()
    }
}

impl GetProperty for u32 {
    fn get_property<'a>(&self, ctxt: &'a ContextRef, this: &Value) -> Option<Local<'a, Value>> {
        ctxt.bind(unsafe { ffi::JS_GetPropertyUint32(ctxt.as_ptr(), this.raw(), *self) })
            .check_undefined()
    }
}

impl GetProperty for Local<'_, ffi::JSAtom> {
    fn get_property<'a>(&self, ctxt: &'a ContextRef, this: &Value) -> Option<Local<'a, Value>> {
        ctxt.bind(unsafe {
            ffi::JS_GetPropertyInternal(ctxt.as_ptr(), this.raw(), **self, this.raw(), FALSE)
        })
        .check_undefined()
    }
}

/// Set a property value on an object.
pub trait SetProperty {
    /// Set a property value on an object.
    fn set_property<T: NewValue>(
        &self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
    ) -> Result<bool, Error>;
}

impl SetProperty for u32 {
    fn set_property<T: NewValue>(
        &self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
    ) -> Result<bool, Error> {
        let ret = unsafe {
            ffi::JS_SetPropertyUint32(ctxt.as_ptr(), this.raw(), *self, val.new_value(ctxt))
        };

        ctxt.check_error(ret).and_then(|ret| match ret {
            TRUE => Ok(true),
            FALSE => Ok(false),
            _ => Err(format_err!("unexpected result: {}", ret)),
        })
    }
}

impl SetProperty for i64 {
    fn set_property<T: NewValue>(
        &self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
    ) -> Result<bool, Error> {
        let ret = unsafe {
            ffi::JS_SetPropertyInt64(ctxt.as_ptr(), this.raw(), *self, val.new_value(ctxt))
        };

        ctxt.check_error(ret).and_then(|ret| match ret {
            TRUE => Ok(true),
            FALSE => Ok(false),
            _ => Err(format_err!("unexpected result: {}", ret)),
        })
    }
}

impl SetProperty for &str {
    fn set_property<T: NewValue>(
        &self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
    ) -> Result<bool, Error> {
        ctxt.check_bool(unsafe {
            ffi::JS_SetPropertyStr(
                ctxt.as_ptr(),
                this.raw(),
                CString::new(*self)?.as_ptr(),
                val.new_value(ctxt),
            )
        })
    }
}

impl SetProperty for Local<'_, ffi::JSAtom> {
    fn set_property<T: NewValue>(
        &self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
    ) -> Result<bool, Error> {
        ctxt.check_bool(unsafe {
            ffi::JS_SetPropertyInternal(
                ctxt.as_ptr(),
                this.raw(),
                **self,
                val.new_value(ctxt),
                ffi::JS_PROP_THROW as i32,
            )
        })
    }
}

/// Check if a property on an object.
pub trait HasProperty {
    /// Check if a property on an object.
    fn has_property(self, ctxt: &ContextRef, this: &Value) -> Result<bool, Error>;
}

impl<'a, T> HasProperty for T
where
    T: NewAtom,
{
    fn has_property(self, ctxt: &ContextRef, this: &Value) -> Result<bool, Error> {
        let atom = self.new_atom(ctxt);
        let ret = unsafe { ffi::JS_HasProperty(ctxt.as_ptr(), this.raw(), atom) };

        ctxt.free_atom(atom);
        ctxt.check_bool(ret)
    }
}

/// Delete a property on an object.
pub trait DeleteProperty {
    /// Delete a property on an object.
    ///
    /// It returns a `bool` indicating whether or not the property was successfully deleted.
    fn delete_property(self, ctxt: &ContextRef, this: &Value) -> Result<bool, Error>;
}

impl<'a, T> DeleteProperty for T
where
    T: NewAtom,
{
    fn delete_property(self, ctxt: &ContextRef, this: &Value) -> Result<bool, Error> {
        let atom = self.new_atom(ctxt);
        let ret = unsafe {
            ffi::JS_DeleteProperty(ctxt.as_ptr(), this.raw(), atom, ffi::JS_PROP_THROW as i32)
        };

        ctxt.free_atom(atom);
        ctxt.check_bool(ret)
    }
}

/// Defines a new property directly on an object, or modifies an existing property on an object.
pub trait DefineProperty {
    /// Defines a new property directly on an object, or modifies an existing property on an object.
    fn define_property(
        self,
        ctxt: &ContextRef,
        this: &Value,
        val: Option<Value>,
        getter: Option<&Value>,
        setter: Option<&Value>,
        flags: Prop,
    ) -> Result<bool, Error>;
}

impl<'a, T> DefineProperty for T
where
    T: NewAtom,
{
    fn define_property(
        self,
        ctxt: &ContextRef,
        this: &Value,
        val: Option<Value>,
        getter: Option<&Value>,
        setter: Option<&Value>,
        mut flags: Prop,
    ) -> Result<bool, Error> {
        let atom = self.new_atom(ctxt);
        if val.is_some() {
            flags |= Prop::HAS_VALUE;
        }
        if getter.is_some() {
            flags |= Prop::HAS_GET;
        }
        if setter.is_some() {
            flags |= Prop::HAS_SET;
        }
        let ret = unsafe {
            ffi::JS_DefineProperty(
                ctxt.as_ptr(),
                this.raw(),
                atom,
                val.map_or_else(|| Value::undefined().raw(), |v| v.raw()),
                getter.map_or_else(|| Value::undefined().raw(), |v| v.raw()),
                setter.map_or_else(|| Value::undefined().raw(), |v| v.raw()),
                flags.bits as i32,
            )
        };
        ctxt.free_atom(atom);
        ctxt.check_bool(ret)
    }
}

pub trait DefinePropertyValue {
    /// Defines a new property with value directly on an object, or modifies an existing property on an object.
    fn define_property<T: NewValue>(
        self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
        flags: Prop,
    ) -> Result<bool, Error>;
}

impl DefinePropertyValue for u32 {
    fn define_property<T: NewValue>(
        self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
        flags: Prop,
    ) -> Result<bool, Error> {
        ctxt.check_bool(unsafe {
            ffi::JS_DefinePropertyValueUint32(
                ctxt.as_ptr(),
                this.raw(),
                self,
                val.new_value(ctxt),
                flags.bits as i32,
            )
        })
    }
}

impl DefinePropertyValue for &'_ str {
    fn define_property<T: NewValue>(
        self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
        flags: Prop,
    ) -> Result<bool, Error> {
        ctxt.check_bool(unsafe {
            ffi::JS_DefinePropertyValueStr(
                ctxt.as_ptr(),
                this.raw(),
                CString::new(self)?.as_ptr(),
                val.new_value(ctxt),
                flags.bits as i32,
            )
        })
    }
}

impl DefinePropertyValue for Local<'_, ffi::JSAtom> {
    fn define_property<T: NewValue>(
        self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
        flags: Prop,
    ) -> Result<bool, Error> {
        ctxt.check_bool(unsafe {
            ffi::JS_DefinePropertyValue(
                ctxt.as_ptr(),
                this.raw(),
                *self,
                val.new_value(ctxt),
                flags.bits as i32,
            )
        })
    }
}

pub trait DefinePropertyGetSet {
    /// Defines a new property with getter and setter directly on an object, or modifies an existing property on an object.
    fn define_property(
        self,
        ctxt: &ContextRef,
        this: &Value,
        getter: Option<&Value>,
        setter: Option<&Value>,
        flags: Prop,
    ) -> Result<bool, Error>;
}

impl<T> DefinePropertyGetSet for T
where
    T: NewAtom,
{
    fn define_property(
        self,
        ctxt: &ContextRef,
        this: &Value,
        getter: Option<&Value>,
        setter: Option<&Value>,
        mut flags: Prop,
    ) -> Result<bool, Error> {
        let atom = self.new_atom(ctxt);
        if getter.is_some() {
            flags |= Prop::HAS_GET;
        }
        if setter.is_some() {
            flags |= Prop::HAS_SET;
        }
        let ret = unsafe {
            ffi::JS_DefinePropertyGetSet(
                ctxt.as_ptr(),
                this.raw(),
                atom,
                getter.map_or_else(|| Value::undefined().raw(), |v| v.raw()),
                setter.map_or_else(|| Value::undefined().raw(), |v| v.raw()),
                flags.bits as i32,
            )
        };
        ctxt.free_atom(atom);
        ctxt.check_bool(ret)
    }
}

/// A property descriptor is a record with some of the following attributes:
#[derive(Debug, Default)]
pub struct Descriptor<'a> {
    /// `true` if and only if the value associated with the property may be changed (data descriptors only).
    pub writable: bool,
    /// The value associated with the property (data descriptors only).
    pub value: Option<Local<'a, Value>>,
    /// A function which serves as a getter for the property.
    pub getter: Option<Local<'a, Value>>,
    /// A function which serves as a setter for the property.
    pub setter: Option<Local<'a, Value>>,
    /// `true` if and only if the type of this property descriptor may be changed
    /// and if the property may be deleted from the corresponding object.
    pub configurable: bool,
    /// `true` if and only if this property shows up during enumeration of the properties on the corresponding object.
    pub enumerable: bool,
}

impl<'a> Local<'a, Value> {
    /// Returns an array of a given object's own property names, in the same order as we get with a normal loop.
    pub fn keys(&self) -> Result<Option<Vec<Atom>>, Error> {
        self.ctxt
            .get_own_property_names(self, Names::ENUM_ONLY | Names::STRING)
    }

    /// Returns an array of all properties (including non-enumerable properties except for those which use Symbol)
    /// found directly in a given object.
    pub fn get_own_property_names(&self) -> Result<Option<Vec<Atom>>, Error> {
        self.ctxt
            .get_own_property_names(self, Names::STRING | Names::SYMBOL)
    }

    /// Returns a property descriptor for an own property
    /// (that is, one directly present on an object and not in the object's prototype chain) of a given object.
    pub fn get_own_property_descriptor<T: NewAtom>(
        &self,
        prop: T,
    ) -> Result<Option<Descriptor>, Error> {
        self.ctxt.get_own_property_descriptor(self, prop)
    }

    /// Get a property value on an object.
    pub fn get_property<T: GetProperty>(&self, prop: T) -> Option<Local<Value>> {
        self.ctxt.get_property(self, prop)
    }

    /// Set a property value on an object.
    pub fn set_property<T: SetProperty, V: NewValue>(
        &self,
        prop: T,
        val: V,
    ) -> Result<bool, Error> {
        self.ctxt.set_property(self, prop, val)
    }

    /// Check if a property on an object.
    pub fn has_property<T: HasProperty>(&self, prop: T) -> Result<bool, Error> {
        self.ctxt.has_property(self, prop)
    }

    /// Delete a property on an object.
    ///
    /// It returns a `bool` indicating whether or not the property was successfully deleted.
    pub fn delete_property<T: DeleteProperty>(&self, prop: T) -> Result<bool, Error> {
        self.ctxt.delete_property(self, prop)
    }

    /// Defines a new property directly on an object, or modifies an existing property on an object.
    pub fn define_property<T: DefineProperty>(
        &self,
        prop: T,
        val: Option<Value>,
        getter: Option<&Value>,
        setter: Option<&Value>,
        flags: Prop,
    ) -> Result<bool, Error> {
        self.ctxt
            .define_property(self, prop, val, getter, setter, flags)
    }

    /// Defines a new property with value directly on an object,
    /// or modifies an existing property on an object.
    pub fn define_property_value<T: DefinePropertyValue, V: NewValue>(
        &self,
        prop: T,
        val: V,
        flags: Prop,
    ) -> Result<bool, Error> {
        self.ctxt.define_property_value(self, prop, val, flags)
    }

    /// Defines a new property with getter and setter directly on an object,
    /// or modifies an existing property on an object.
    pub fn define_property_get_set<T: DefinePropertyGetSet>(
        &self,
        prop: T,
        getter: Option<&Value>,
        setter: Option<&Value>,
        flags: Prop,
    ) -> Result<bool, Error> {
        self.ctxt
            .define_property_get_set(self, prop, getter, setter, flags)
    }

    /// Check if an object is extensible (whether it can have new properties added to it).
    pub fn is_extensible(&self) -> Result<bool, Error> {
        self.ctxt.is_extensible(self)
    }

    /// Prevents new properties from ever being added to an object (i.e. prevents future extensions to the object).
    pub fn prevent_extensions(&self) -> Result<bool, Error> {
        self.ctxt.prevent_extensions(self)
    }
}

impl ContextRef {
    /// Returns an array of all properties (including non-enumerable properties except for those which use Symbol)
    /// found directly in a given object.
    pub fn get_own_property_names(
        &self,
        value: &Value,
        flags: Names,
    ) -> Result<Option<Vec<Atom>>, Error> {
        let mut ptab = ptr::null_mut();
        let mut count = 0;

        self.check_error(unsafe {
            ffi::JS_GetOwnPropertyNames(
                self.as_ptr(),
                &mut ptab,
                &mut count,
                value.raw(),
                flags.bits() as i32,
            )
        })
        .map(|_| {
            let names = unsafe { slice::from_raw_parts(ptab, count as usize) };
            let names = names
                .iter()
                .map(|prop| self.bind_atom(prop.atom))
                .collect::<Vec<_>>();

            unsafe { ffi::js_free(self.as_ptr(), ptab as *mut _) }

            Some(names)
        })
    }

    /// Returns a property descriptor for an own property
    /// (that is, one directly present on an object and not in the object's prototype chain) of a given object.
    pub fn get_own_property_descriptor<T: NewAtom>(
        &self,
        value: &Value,
        prop: T,
    ) -> Result<Option<Descriptor>, Error> {
        let atom = prop.new_atom(self);
        let mut desc = MaybeUninit::<ffi::JSPropertyDescriptor>::uninit();
        let res =
            unsafe { ffi::JS_GetOwnProperty(self.as_ptr(), desc.as_mut_ptr(), value.raw(), atom) };
        self.free_atom(atom);

        self.check_bool(res).map(|exists| {
            if exists {
                let desc = unsafe { desc.assume_init() };
                let flags = Prop::from_bits_truncate(desc.flags as u32);

                Some(Descriptor {
                    writable: flags.contains(Prop::WRITABLE),
                    value: Value::new(desc.value).map(|v| self.bind(v)),
                    getter: Value::new(desc.getter).map(|v| self.bind(v)),
                    setter: Value::new(desc.setter).map(|v| self.bind(v)),
                    configurable: flags.contains(Prop::CONFIGURABLE),
                    enumerable: flags.contains(Prop::ENUMERABLE),
                })
            } else {
                None
            }
        })
    }

    /// Get a property value on an object.
    pub fn get_property<T: GetProperty>(&self, this: &Value, prop: T) -> Option<Local<Value>> {
        prop.get_property(self, this)
    }

    /// Set a property value on an object.
    pub fn set_property<T: SetProperty, V: NewValue>(
        &self,
        this: &Value,
        prop: T,
        val: V,
    ) -> Result<bool, Error> {
        prop.set_property(self, this, val)
    }

    /// Check if a property on an object.
    pub fn has_property<T: HasProperty>(&self, this: &Value, prop: T) -> Result<bool, Error> {
        prop.has_property(self, this)
    }

    /// Delete a property on an object.
    ///
    /// It returns a `bool` indicating whether or not the property was successfully deleted.
    pub fn delete_property<T: DeleteProperty>(&self, this: &Value, prop: T) -> Result<bool, Error> {
        prop.delete_property(self, this)
    }

    /// Defines a new property directly on an object, or modifies an existing property on an object.
    pub fn define_property<T: DefineProperty>(
        &self,
        this: &Value,
        prop: T,
        val: Option<Value>,
        getter: Option<&Value>,
        setter: Option<&Value>,
        flags: Prop,
    ) -> Result<bool, Error> {
        prop.define_property(self, this, val, getter, setter, flags)
    }

    /// Defines a new property with value directly on an object,
    /// or modifies an existing property on an object.
    pub fn define_property_value<T: DefinePropertyValue, V: NewValue>(
        &self,
        this: &Value,
        prop: T,
        val: V,
        flags: Prop,
    ) -> Result<bool, Error> {
        prop.define_property(self, this, val, flags)
    }

    /// Defines a new property with getter and setter directly on an object,
    /// or modifies an existing property on an object.
    pub fn define_property_get_set<T: DefinePropertyGetSet>(
        &self,
        this: &Value,
        prop: T,
        getter: Option<&Value>,
        setter: Option<&Value>,
        flags: Prop,
    ) -> Result<bool, Error> {
        prop.define_property(self, this, getter, setter, flags)
    }

    /// Check if an object is extensible (whether it can have new properties added to it).
    pub fn is_extensible(&self, obj: &Value) -> Result<bool, Error> {
        self.check_bool(unsafe { ffi::JS_IsExtensible(self.as_ptr(), obj.raw()) })
    }

    /// Prevents new properties from ever being added to an object (i.e. prevents future extensions to the object).
    pub fn prevent_extensions(&self, obj: &Value) -> Result<bool, Error> {
        self.check_bool(unsafe { ffi::JS_PreventExtensions(self.as_ptr(), obj.raw()) })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, ErrorKind, Eval, Runtime};

    #[test]
    fn set_property() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let obj = ctxt
            .eval_script("new Object();", "<evalScript>", Eval::GLOBAL)
            .unwrap();

        assert!(!obj.has_property("foo").unwrap());
        assert!(obj.get_property("foo").is_none());

        assert!(obj.set_property("foo", "bar").unwrap());
        assert!(obj.has_property("foo").unwrap());
        assert_eq!(obj.get_property("foo").unwrap().to_string(), "bar");

        assert_eq!(
            obj.get_own_property_names()
                .unwrap()
                .unwrap()
                .into_iter()
                .map(|name| name.to_cstr().to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            vec!["foo"]
        );

        let desc = obj.get_own_property_descriptor("foo").unwrap().unwrap();
        assert!(desc.writable);
        assert_eq!(desc.value.unwrap().to_string(), "bar");
        assert!(desc.getter.is_none());
        assert!(desc.setter.is_none());
        assert!(desc.configurable);
        assert!(desc.enumerable);

        assert!(obj.delete_property("foo").unwrap());
        assert!(!obj.has_property("foo").unwrap());
    }

    #[test]
    fn extensible() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let obj = ctxt
            .eval_script("new Object();", "<evalScript>", Eval::GLOBAL)
            .unwrap();

        assert!(obj.is_extensible().unwrap());
        assert!(obj.prevent_extensions().unwrap());
        assert!(!obj.is_extensible().unwrap());

        assert_eq!(
            obj.set_property("foo", "bar")
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            ErrorKind::TypeError("object is not extensible".into(), None)
        );
    }
}
