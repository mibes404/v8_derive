//! This module contains the `TryFromValue` trait which is used to convert a `v8::Value` into a Rust type.

#[cfg(feature = "json")]
use crate::json::v8_to_json_value;
use crate::{
    errors,
    helpers::{
        try_as_bool, try_as_f32, try_as_f64, try_as_hashmap, try_as_i32, try_as_i64, try_as_i8, try_as_string,
        try_as_u32,
    },
    try_as_vec,
};
use std::{collections::HashMap, hash::BuildHasher};

/// The `TryFromValue` trait is used to convert a `v8::Value` into a Rust type.
pub trait TryFromValue {
    /// Converts a `v8::Value` into a Rust type.
    ///
    /// # Errors
    /// In case of conversion errors, or if the value is not supported, an error is returned.
    fn try_from_value(
        input: &v8::Local<'_, v8::Value>,
        scope: &mut v8::PinScope<'_, '_>,
    ) -> errors::Result<Self>
    where
        Self: Sized;
}

impl<T> TryFromValue for Vec<T>
where
    T: TryFromValue,
{
    fn try_from_value(
        input: &v8::Local<'_, v8::Value>,
        scope: &mut v8::PinScope<'_, '_>,
    ) -> errors::Result<Self> {
        try_as_vec(input, scope)
    }
}

impl<T, S> TryFromValue for HashMap<String, T, S>
where
    T: TryFromValue,
    S: BuildHasher + Default,
{
    fn try_from_value(
        input: &v8::Local<'_, v8::Value>,
        scope: &mut v8::PinScope<'_, '_>,
    ) -> errors::Result<Self> {
        try_as_hashmap(input, scope)
    }
}

impl<T> TryFromValue for Option<T>
where
    T: TryFromValue,
{
    fn try_from_value(
        input: &v8::Local<'_, v8::Value>,
        scope: &mut v8::PinScope<'_, '_>,
    ) -> errors::Result<Self> {
        if input.is_null_or_undefined() {
            return Ok(None);
        }

        let value = T::try_from_value(input, scope)?;
        Ok(Some(value))
    }
}

#[cfg(feature = "json")]
impl TryFromValue for serde_json::Value {
    fn try_from_value(
        input: &v8::Local<'_, v8::Value>,
        scope: &mut v8::PinScope<'_, '_>,
    ) -> errors::Result<Self> {
        let value = v8_to_json_value(scope, *input)?;
        Ok(value)
    }
}

macro_rules! impl_try_from_value {
    ($($t:ty => $func:ident),*) => {
        $(
            impl TryFromValue for $t {
                fn try_from_value<'a>(
                    input: &'a v8::Local<'a, v8::Value>,
                    scope: &mut v8::PinScope<'_, '_>,
                ) -> errors::Result<Self> {
                    $func(input, scope)
                }
            }
        )*
    };
}

impl_try_from_value! {
    bool => try_as_bool,
    String => try_as_string,
    i8 => try_as_i8,
    i32 => try_as_i32,
    i64 => try_as_i64,
    f64 => try_as_f64,
    u32 => try_as_u32,
    f32 => try_as_f32
}

#[cfg(test)]
mod tests {
    use crate::{self as v8_derive, from::TryFromValue, setup};
    use std::collections::HashMap;
    use v8::{ContextOptions, CreateParams, Local, Value};
    use v8_derive_macros::FromValue;

    #[derive(Debug, FromValue)]
    struct SimpleObject {
        yes_no: bool,
        name: String,
        age: i32,
        opt: Option<i32>,
        avg: f64,
    }

    #[derive(FromValue)]
    struct OptionalObject {
        opt: Option<i32>,
    }

    #[derive(Debug, FromValue)]
    struct ParentObject {
        nested: SimpleObject,
    }

    #[derive(FromValue)]
    struct ObjectWithVec {
        vec: Vec<i32>,
    }

    #[test]
    fn should_be_able_to_handle_incomplete_values() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // null
        let js_null = v8::null(scope).into();
        SimpleObject::try_from_value(&js_null, scope).expect_err("can't deserialize null");

        // missing mandatory field
        let object = v8::Object::new(scope);
        let js_key = v8::String::new(scope, "yes_no").unwrap().into();
        let js_bool_val = v8::Boolean::new(scope, true).into();
        object.set(scope, js_key, js_bool_val);
        let object: Local<'_, Value> = object.cast();
        SimpleObject::try_from_value(&object, scope).expect("deserialize failed");
    }

    #[test]
    fn should_be_able_to_parse_primitives() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // bool
        let js_bool_val = v8::Boolean::new(scope, true).into();
        let bool_val = bool::try_from_value(&js_bool_val, scope).unwrap();
        assert!(bool_val);

        #[cfg(feature = "json")]
        {
            let json_val = serde_json::Value::try_from_value(&js_bool_val, scope).unwrap();
            assert_eq!(json_val, serde_json::Value::Bool(true));
        }

        // String
        let js_string_val = v8::String::new(scope, "Hello, World!").unwrap().into();
        let string_val = String::try_from_value(&js_string_val, scope).unwrap();
        assert_eq!(string_val, "Hello, World!");

        #[cfg(feature = "json")]
        {
            let json_val = serde_json::Value::try_from_value(&js_string_val, scope).unwrap();
            assert_eq!(json_val, serde_json::Value::String("Hello, World!".to_string()));
        }

        // i32
        let js_int_val = v8::Integer::new(scope, 42).into();
        let int_val = i32::try_from_value(&js_int_val, scope).unwrap();
        assert_eq!(int_val, 42);

        #[cfg(feature = "json")]
        {
            let json_val = serde_json::Value::try_from_value(&js_int_val, scope).unwrap();
            assert_eq!(json_val, serde_json::Value::Number(serde_json::Number::from(42)));
        }

        // Vec<i32>
        let js_array = v8::Array::new(scope, 3);
        let js_val_1 = v8::Integer::new(scope, 1);
        js_array.set_index(scope, 0, js_val_1.into());
        let js_val_2 = v8::Integer::new(scope, 2);
        js_array.set_index(scope, 1, js_val_2.into());
        let js_val_3 = v8::Integer::new(scope, 3);
        js_array.set_index(scope, 2, js_val_3.into());
        let array_val = Vec::<i32>::try_from_value(&js_array.into(), scope).unwrap();
        assert_eq!(array_val, vec![1, 2, 3]);

        #[cfg(feature = "json")]
        {
            let json_val = serde_json::Value::try_from_value(&js_array.into(), scope).unwrap();
            assert_eq!(
                json_val,
                serde_json::Value::Array(vec![
                    serde_json::Value::Number(serde_json::Number::from(1)),
                    serde_json::Value::Number(serde_json::Number::from(2)),
                    serde_json::Value::Number(serde_json::Number::from(3))
                ])
            );
        }

        // Option<i32>
        let js_null = v8::null(scope).into();
        let null_val = Option::<i32>::try_from_value(&js_null, scope).unwrap();
        assert!(null_val.is_none());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn should_be_able_to_parse_a_simple_object() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let object = v8::Object::new(scope);
        // yes_no
        let js_key = v8::String::new(scope, "yes_no").unwrap().into();
        let js_bool_val = v8::Boolean::new(scope, true).into();
        object.set(scope, js_key, js_bool_val);
        // name
        let js_key = v8::String::new(scope, "name").unwrap().into();
        let js_bool_val = v8::String::new(scope, "Marcel").unwrap().into();
        object.set(scope, js_key, js_bool_val);
        // age
        let js_key = v8::String::new(scope, "age").unwrap().into();
        let js_bool_val = v8::Integer::new(scope, 30).into();
        object.set(scope, js_key, js_bool_val);
        // opt
        let js_key = v8::String::new(scope, "opt").unwrap().into();
        let js_bool_val = v8::Integer::new(scope, 42).into();
        object.set(scope, js_key, js_bool_val);
        // avg
        let js_key = v8::String::new(scope, "avg").unwrap().into();
        let js_bool_val = v8::Number::new(scope, 42.42).into();
        object.set(scope, js_key, js_bool_val);

        let object: Local<'_, Value> = object.cast();
        let s: SimpleObject = SimpleObject::try_from_value(&object, scope).expect("failed to deserialize");
        assert!(s.yes_no);
        assert_eq!(s.name, "Marcel");
        assert_eq!(s.age, 30);
        assert_eq!(s.opt, Some(42));
        assert_eq!(s.avg, 42.42);
    }

    #[test]
    fn should_be_able_to_handle_optional_fields() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // Some value
        let object = v8::Object::new(scope);
        let js_key = v8::String::new(scope, "opt").unwrap().into();
        let js_bool_val = v8::Integer::new(scope, 42).into();
        object.set(scope, js_key, js_bool_val);

        let object: Local<'_, Value> = object.cast();
        let s: OptionalObject = OptionalObject::try_from_value(&object, scope).expect("failed to deserialize");
        assert_eq!(s.opt, Some(42));

        // Null value
        let object = v8::Object::new(scope);
        let js_key = v8::String::new(scope, "opt").unwrap().into();
        let js_bool_val = v8::null(scope).into();
        object.set(scope, js_key, js_bool_val);

        let object: Local<'_, Value> = object.cast();
        let s: OptionalObject = OptionalObject::try_from_value(&object, scope).expect("failed to deserialize");
        assert_eq!(s.opt, None);

        // Missing value
        let object = v8::Object::new(scope);
        let object: Local<'_, Value> = object.cast();
        let s: OptionalObject = OptionalObject::try_from_value(&object, scope).expect("failed to deserialize");
        assert_eq!(s.opt, None);
    }

    #[test]
    fn should_be_able_to_parse_nested_objects() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // Child object
        let object = v8::Object::new(scope);
        // yes_no
        let js_key = v8::String::new(scope, "yes_no").unwrap().into();
        let js_bool_val = v8::Boolean::new(scope, true).into();
        object.set(scope, js_key, js_bool_val);
        // name
        let js_key = v8::String::new(scope, "name").unwrap().into();
        let js_bool_val = v8::String::new(scope, "Marcel").unwrap().into();
        object.set(scope, js_key, js_bool_val);
        // age
        let js_key = v8::String::new(scope, "age").unwrap().into();
        let js_bool_val = v8::Integer::new(scope, 30).into();
        object.set(scope, js_key, js_bool_val);
        // opt
        let js_key = v8::String::new(scope, "opt").unwrap().into();
        let js_bool_val = v8::Integer::new(scope, 42).into();
        object.set(scope, js_key, js_bool_val);
        // avg
        let js_key = v8::String::new(scope, "avg").unwrap().into();
        let js_bool_val = v8::Number::new(scope, 42.42).into();
        object.set(scope, js_key, js_bool_val);

        // Parent object
        let parent_object = v8::Object::new(scope);
        let js_key = v8::String::new(scope, "nested").unwrap().into();
        parent_object.set(scope, js_key, object.into());
        let parent_object: Local<'_, Value> = parent_object.cast();

        // Deserialize
        let p: ParentObject = ParentObject::try_from_value(&parent_object, scope).expect("failed to deserialize");
        let s = p.nested;
        assert!(s.yes_no);
    }

    #[test]
    fn can_deserialize_an_object_with_a_vec() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // Child object
        let object = v8::Object::new(scope);
        let js_key = v8::String::new(scope, "vec").unwrap().into();
        let js_array = v8::Array::new(scope, 3);
        let js_val_1 = v8::Integer::new(scope, 1);
        js_array.set_index(scope, 0, js_val_1.into());
        let js_val_2 = v8::Integer::new(scope, 2);
        js_array.set_index(scope, 1, js_val_2.into());
        let js_val_3 = v8::Integer::new(scope, 3);
        js_array.set_index(scope, 2, js_val_3.into());
        object.set(scope, js_key, js_array.into());
        let object: Local<'_, Value> = object.cast();

        // Deserialize
        let p: ObjectWithVec = ObjectWithVec::try_from_value(&object, scope).expect("failed to deserialize");
        assert_eq!(p.vec, vec![1, 2, 3]);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn should_be_able_to_parse_a_simple_object_to_hashmap() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let object = v8::Object::new(scope);
        // yes_no
        let js_key = v8::String::new(scope, "yes_no").unwrap().into();
        let js_bool_val = v8::Boolean::new(scope, true).into();
        object.set(scope, js_key, js_bool_val);
        // name
        let js_key = v8::String::new(scope, "name").unwrap().into();
        let js_bool_val = v8::String::new(scope, "Marcel").unwrap().into();
        object.set(scope, js_key, js_bool_val);
        // age
        let js_key = v8::String::new(scope, "age").unwrap().into();
        let js_bool_val = v8::Integer::new(scope, 30).into();
        object.set(scope, js_key, js_bool_val);
        // opt
        let js_key = v8::String::new(scope, "opt").unwrap().into();
        let js_bool_val = v8::Integer::new(scope, 42).into();
        object.set(scope, js_key, js_bool_val);
        // avg
        let js_key = v8::String::new(scope, "avg").unwrap().into();
        let js_bool_val = v8::Number::new(scope, 42.42).into();
        object.set(scope, js_key, js_bool_val);

        let object: Local<'_, Value> = object.cast();
        let s: HashMap<String, String> = HashMap::try_from_value(&object, scope).expect("failed to deserialize");
        assert_eq!(s.get("yes_no"), Some(&"true".to_string()));
        assert_eq!(s.get("name"), Some(&"Marcel".to_string()));
        assert_eq!(s.get("age"), Some(&"30".to_string()));
        assert_eq!(s.get("opt"), Some(&"42".to_string()));
        assert_eq!(s.get("avg"), Some(&"42.42".to_string()));
    }
}
