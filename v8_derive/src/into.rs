//! This module provides a trait to convert a Rust type into a v8 Value.

#[cfg(feature = "json")]
use crate::json::json_to_v8;
use std::{collections::HashMap, hash::BuildHasher};

/// The `IntoValue` trait is used to convert a Rust type into a v8 Value.
pub trait IntoValue {
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value>;
}

/// The `IntoObject` trait is used to convert a Rust type into a v8 Value.
pub trait IntoObject {
    fn into_object<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value>;
}

impl IntoValue for bool {
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        v8::Boolean::new(scope, self).into()
    }
}

impl IntoValue for i32 {
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        v8::Integer::new(scope, self).into()
    }
}

impl IntoValue for u32 {
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        v8::Integer::new_from_unsigned(scope, self).into()
    }
}

impl IntoValue for i64 {
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        v8::BigInt::new_from_i64(scope, self).into()
    }
}

impl IntoValue for f64 {
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        v8::Number::new(scope, self).into()
    }
}

impl IntoValue for f32 {
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        f64::from(self).into_value(scope)
    }
}

impl IntoValue for String {
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        v8::String::new(scope, &self).unwrap_or(v8::String::empty(scope)).into()
    }
}

impl<T> IntoValue for Option<T>
where
    T: IntoValue,
{
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        match self {
            Some(value) => value.into_value(scope),
            None => v8::null(scope).into(),
        }
    }
}

impl<T> IntoValue for Vec<T>
where
    T: IntoValue,
{
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        let l = i32::try_from(self.len()).unwrap_or(i32::MAX);
        let array = v8::Array::new(scope, l);

        for (i, value) in self.into_iter().enumerate() {
            let el: v8::Local<'_, v8::Value> = value.into_value(scope);
            let idx = u32::try_from(i).unwrap_or(u32::MAX);
            array.set_index(scope, idx, el);
        }

        array.into()
    }
}

impl<K, T, S> IntoValue for HashMap<K, T, S>
where
    K: IntoValue,
    T: IntoValue,
    S: BuildHasher,
{
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        let object = v8::Map::new(scope);

        for (key, value) in self {
            let js_key = key.into_value(scope);
            let js_val = value.into_value(scope);
            object.set(scope, js_key, js_val);
        }

        object.into()
    }
}

impl<T, S> IntoValue for HashMap<&str, T, S>
where
    T: IntoValue,
    S: BuildHasher,
{
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        let object = v8::Map::new(scope);

        for (key, value) in self {
            let js_key = v8::String::new(scope, key).unwrap().into();
            let js_val = value.into_value(scope);
            object.set(scope, js_key, js_val);
        }

        object.into()
    }
}

impl<K, T, S> IntoObject for HashMap<K, T, S>
where
    K: IntoValue,
    T: IntoValue,
    S: BuildHasher,
{
    fn into_object<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        let object = v8::Object::new(scope);

        for (key, value) in self {
            let js_key = key.into_value(scope);
            let js_val = value.into_value(scope);
            object.set(scope, js_key, js_val);
        }

        object.into()
    }
}

#[cfg(feature = "json")]
impl IntoValue for serde_json::Value {
    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
        json_to_v8(scope, self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        into::{IntoObject, IntoValue},
        setup, TryFromValue,
    };
    use std::collections::HashMap;
    use v8::{ContextOptions, CreateParams};

    #[test]
    #[allow(clippy::cast_possible_wrap)]
    fn can_convert_into_an_array() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let array = vec![1, 2, 3, 4, 5];
        let array_value = array.into_value(scope);

        // cast the value to an array
        let array = array_value.try_cast::<v8::Array>().expect("Expected an array");
        assert_eq!(array.length(), 5);
        for i in 0..5 {
            let value = array.get_index(scope, i).unwrap();
            assert_eq!(value.to_int32(scope).unwrap().value(), i as i32 + 1);
        }
    }

    #[test]
    fn can_convert_into_a_js_map() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let map: HashMap<&str, i32> = [("one", 1), ("two", 2), ("three", 3)].into();

        // Convert the map into a JS value
        let map_value: v8::Local<'_, v8::Value> = map.into_value(scope);

        // cast the value to a map
        let map = HashMap::<String, i32>::try_from_value(&map_value, scope).expect("Expected a map");
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("one"), Some(&1));
    }

    #[test]
    fn can_convert_into_a_js_object() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let map: HashMap<String, i32> =
            [("one".to_string(), 1), ("two".to_string(), 2), ("three".to_string(), 3)].into();

        // Convert the map into a JS Object value
        let obj_value: v8::Local<'_, v8::Value> = map.into_object(scope);

        // cast the value to a map
        let map = HashMap::<String, i32>::try_from_value(&obj_value, scope).expect("Expected a map");
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("one"), Some(&1));
    }

    #[test]
    fn can_convert_non_str_keys_into_a_js_object() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let map: HashMap<i32, String> =
            [(1, "one".to_string()), (2, "two".to_string()), (3, "three".to_string())].into();

        // Convert the map into a JS Object value
        let obj_value: v8::Local<'_, v8::Value> = map.into_object(scope);

        // cast the value to a map
        let map = HashMap::<String, String>::try_from_value(&obj_value, scope).expect("Expected a map");
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("1"), Some(&"one".to_string()));
    }

    #[test]
    fn can_convert_non_str_keys_into_a_js_map() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let map: HashMap<i32, String> =
            [(1, "one".to_string()), (2, "two".to_string()), (3, "three".to_string())].into();

        // Convert the map into a JS value
        let obj_value: v8::Local<'_, v8::Value> = map.into_value(scope);

        // cast the value to a map
        let map = HashMap::<String, String>::try_from_value(&obj_value, scope).expect("Expected a map");
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("1"), Some(&"one".to_string()));
    }

    #[test]
    fn can_convert_into_a_string_type_js_map() {
        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let map: HashMap<String, String> = [
            ("one".to_string(), "1".to_string()),
            ("two".to_string(), "2".to_string()),
            ("three".to_string(), "3".to_string()),
        ]
        .into();

        // Convert the map into a JS value
        let map_value: v8::Local<'_, v8::Value> = map.into_value(scope);

        // cast the value to a map
        let map = HashMap::<String, String>::try_from_value(&map_value, scope).expect("Expected a map");
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("one"), Some(&"1".to_string()));
    }

    #[cfg(feature = "json")]
    #[test]
    fn can_convert_json_into_a_js_object() {
        /// The constant 18446744073709552000 represents an approximation of the maximum value of an
        /// unsigned 64-bit integer (u64), which is 2^64 - 1 (or 18446744073709551615).
        /// The slight difference (+385) is due to the limitations of converting a u64 to a String or
        /// f64 in JavaScript, as JavaScript's Number type uses double-precision floating-point
        /// representation, which cannot precisely represent all 64-bit integers.
        const MAX_JS_UINT: &str = "18446744073709552000";

        let json = serde_json::json!({
            "name": "John",
            "age": 30,
            "very_large_number": u64::MAX,
        });

        setup::setup_test();
        let isolate = &mut v8::Isolate::new(CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // Convert the JSON into a JS Object value
        let obj_value: v8::Local<'_, v8::Value> = json.into_value(scope);

        // cast the value to a map
        let map = HashMap::<String, String>::try_from_value(&obj_value, scope).expect("Expected a map");
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("name"), Some(&"John".to_string()));
        assert_eq!(map.get("age"), Some(&"30".to_string()));
        assert_eq!(map.get("very_large_number"), Some(&MAX_JS_UINT.to_string()));
    }
}
