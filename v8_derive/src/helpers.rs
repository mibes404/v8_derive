use crate::{errors, from::TryFromValue};
use std::{collections::HashMap, hash::BuildHasher};
use v8::GetPropertyNamesArgs;

pub fn get_field_as<T>(
    field_name: &str,
    input: &v8::Local<'_, v8::Value>,
    scope: &mut v8::PinScope<'_, '_>,
    parse_fn: ParseFn<T>,
) -> errors::Result<T> {
    if !input.is_object() {
        return Err(errors::Error::ExpectedObject);
    }

    let js_object: v8::Local<v8::Object> = input.try_cast()?;
    let js_key = v8::String::new(scope, field_name)
        .map(Into::into)
        .ok_or(errors::Error::InvalidField(field_name.to_string()))?;
    let js_value = js_object
        .get(scope, js_key)
        .ok_or(errors::Error::FieldNotFound(field_name.to_string()))?;

    parse_fn(&js_value, scope)
}

pub fn get_optional_field_as<T>(
    field_name: &str,
    input: &v8::Local<'_, v8::Value>,
    scope: &mut v8::PinScope<'_, '_>,
    parse_fn: ParseFn<T>,
) -> errors::Result<Option<T>> {
    if !input.is_object() {
        return Err(errors::Error::ExpectedObject);
    }

    let js_object: v8::Local<v8::Object> = input.try_cast()?;
    let js_key = v8::String::new(scope, field_name)
        .map(Into::into)
        .ok_or(errors::Error::InvalidField(field_name.to_string()))?;
    let js_value = js_object.get(scope, js_key);

    // field not found
    let Some(js_value) = js_value else {
        return Ok(None);
    };

    // check for null
    if js_value.is_null_or_undefined() {
        return Ok(None);
    }

    let inner_value = parse_fn(&js_value, scope)?;
    Ok(Some(inner_value))
}

pub type ParseFn<T> = fn(&v8::Local<'_, v8::Value>, &mut v8::PinScope<'_, '_>) -> errors::Result<T>;

pub fn try_as_bool(input: &v8::Local<'_, v8::Value>, scope: &mut v8::PinScope<'_, '_>) -> errors::Result<bool> {
    // do not check if boolean, JS will answer something, using
    // the boolean values will do the logic from JS exported to the Rust translation
    Ok(input.boolean_value(scope))
}

pub fn try_as_string(input: &v8::Local<'_, v8::Value>, scope: &mut v8::PinScope<'_, '_>) -> errors::Result<String> {
    // try to convert the value to String anyway
    Ok(input.to_rust_string_lossy(scope))
}

pub fn try_as_i32(input: &v8::Local<'_, v8::Value>, scope: &mut v8::PinScope<'_, '_>) -> errors::Result<i32> {
    // use the framework to get the internal convertion
    input.int32_value(scope).ok_or(errors::Error::ExpectedI32)
}

pub fn try_as_u32(input: &v8::Local<'_, v8::Value>, scope: &mut v8::PinScope<'_, '_>) -> errors::Result<u32> {
    if input.is_uint32() {
        return input.uint32_value(scope).ok_or(errors::Error::ExpectedU32);
    }
    if input.is_null_or_undefined() {
        return Ok(0);
    }
    // use the framework to get the internal conversion
    u32::try_from(input.to_big_int(scope).ok_or(errors::Error::ExpectedU32)?.i64_value().0)
        .map_err(|_| errors::Error::OutOfRange)
}

pub fn try_as_i64(input: &v8::Local<'_, v8::Value>, scope: &mut v8::PinScope<'_, '_>) -> errors::Result<i64> {
    // use the framework to get the internal convertion
    let i = input.to_big_int(scope).ok_or(errors::Error::ExpectedI64)?;
    Ok(i.i64_value().0)
}

pub fn try_as_f64(input: &v8::Local<'_, v8::Value>, scope: &mut v8::PinScope<'_, '_>) -> errors::Result<f64> {
    // use the framework to get the internal convertion
    input.number_value(scope).ok_or(errors::Error::ExpectedF64)
}

#[allow(clippy::cast_possible_truncation)]
pub fn try_as_f32(input: &v8::Local<'_, v8::Value>, scope: &mut v8::PinScope<'_, '_>) -> errors::Result<f32> {
    let i = try_as_f64(input, scope)?;
    Ok(i as f32)
}

pub fn try_as_i8(input: &v8::Local<'_, v8::Value>, scope: &mut v8::PinScope<'_, '_>) -> errors::Result<i8> {
    let i = try_as_i32(input, scope)?;
    i8::try_from(i).map_err(|_| errors::Error::OutOfRange)
}

pub fn try_as_vec<T>(input: &v8::Local<'_, v8::Value>, scope: &mut v8::PinScope<'_, '_>) -> errors::Result<Vec<T>>
where
    T: TryFromValue,
{
    if !input.is_array() {
        return Err(errors::Error::ExpectedArray);
    }

    let array: v8::Local<v8::Array> = input.try_cast()?;
    let length = array.length();

    let mut result = Vec::with_capacity(length as usize);

    for i in 0..length {
        let Some(element) = array.get_index(scope, i) else {
            // this should never happen
            continue;
        };

        let element = T::try_from_value(&element, scope)?;
        result.push(element);
    }

    Ok(result)
}

pub fn try_as_hashmap<T, S>(
    input: &v8::Local<'_, v8::Value>,
    scope: &mut v8::PinScope<'_, '_>,
) -> errors::Result<HashMap<String, T, S>>
where
    T: TryFromValue,
    S: BuildHasher + Default,
{
    if !(input.is_map() || input.is_object()) {
        return Err(errors::Error::ExpectedMap);
    }

    let mut result: HashMap<String, T, S> = HashMap::with_hasher(S::default());

    if input.is_map() {
        let js_map: v8::Local<v8::Map> = input.try_cast()?;
        let js_array = js_map.as_array(scope); // js_array is twice the size of the map; odd indexes are keys, even indexes are values
        for i in (0..js_array.length()).step_by(2) {
            let (Some(key), Some(value)) = (js_array.get_index(scope, i), js_array.get_index(scope, i + 1)) else {
                continue;
            };

            let key = key.to_rust_string_lossy(scope);
            let value = T::try_from_value(&value, scope)?;
            result.insert(key, value);
        }

        return Ok(result);
    }

    // object
    let js_object: v8::Local<v8::Object> = input.try_cast()?;
    let keys = js_object
        .get_own_property_names(scope, GetPropertyNamesArgs::default())
        .ok_or(errors::Error::FailedToGetPropertyNames)?;

    for i in 0..keys.length() {
        let key = keys
            .get_index(scope, i)
            .ok_or(errors::Error::FailedToGetPropertyNames)?;
        let value = js_object
            .get(scope, key)
            .ok_or(errors::Error::FailedToGetPropertyNames)?;
        let value = T::try_from_value(&value, scope)?;
        let key = key.to_rust_string_lossy(scope);
        result.insert(key, value);
    }

    Ok(result)
}

#[cfg(test)]
pub(crate) mod setup {
    use super::{try_as_bool, try_as_i8};
    use crate::{try_as_i32, try_as_u32};
    use std::sync::Once;
    use v8::Value;

    /// Set up global state for a test
    pub(crate) fn setup_test() {
        initialize_once();
    }

    fn initialize_once() {
        static START: Once = Once::new();
        START.call_once(|| {
            v8::V8::set_flags_from_string(
                "--no_freeze_flags_after_init --expose_gc --allow_natives_syntax --turbo_fast_api_calls",
            );
            v8::V8::initialize_platform(v8::new_unprotected_default_platform(0, false).make_shared());
            v8::V8::initialize();
        });
    }

    #[test]
    fn test_try_boolean() {
        // given
        // - v8 is all ok
        setup_test();
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, v8::ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // given
        // - straight true value
        let value: v8::Local<'_, Value> = v8::Boolean::new(scope, true).into();
        // when
        // - try to convert
        let result = try_as_bool(&value, scope);
        // then
        // - expect to be able to convert and result in true
        assert!(result.expect("Expected to be able to convert and be true"));

        // given
        // - straight false value
        let value: v8::Local<'_, Value> = v8::Boolean::new(scope, false).into();
        // when
        // - try to convert
        let result = try_as_bool(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert!(!result.expect("Expected to be able to convert and be false"));
    }

    #[test]
    fn test_try_boolean_from_undefined_null() {
        // given
        // - v8 is all ok
        setup_test();
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, v8::ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // given
        // - undefined value
        let value: v8::Local<'_, Value> = v8::undefined(scope).into();
        // when
        // - try to convert
        let result = try_as_bool(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert!(!result.expect("Expected to be able to convert and be false"));

        // given
        // - null value
        let value: v8::Local<'_, Value> = v8::null(scope).into();
        // when
        // - try to convert
        let result = try_as_bool(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert!(!result.expect("Expected to be able to convert and be false"));
    }

    #[test]
    fn test_try_boolean_from_number() {
        // given
        // - v8 is all ok
        setup_test();
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, v8::ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // given
        // - non zero value
        let value: v8::Local<'_, Value> = v8::Integer::new(scope, 1).into();
        // when
        // - try to convert
        let result = try_as_bool(&value, scope);
        // then
        // - expect to be able to convert and result in true
        assert!(result.expect("Expected to be able to convert and be true"));

        // given
        // - zero value
        let value: v8::Local<'_, Value> = v8::Number::new(scope, 0.0).into();
        // when
        // - try to convert
        let result = try_as_bool(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert!(!result.expect("Expected to be able to convert and be false"));
    }

    #[test]
    fn test_try_boolean_from_string() {
        // given
        // - v8 is all ok
        setup_test();
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, v8::ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // given
        // - empty string
        let value: v8::Local<'_, Value> = v8::String::new(scope, "").unwrap().into();
        // when
        // - try to convert
        let result = try_as_bool(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert!(!result.expect("Expected to be able to convert and be false"));

        // given
        // - non-empty value
        let value: v8::Local<'_, Value> = v8::String::new(scope, "abc").unwrap().into();
        // when
        // - try to convert
        let result = try_as_bool(&value, scope);
        // then
        // - expect to be able to convert and result in true
        assert!(result.expect("Expected to be able to convert and be true"));
    }

    #[test]
    fn test_try_i32_from_string() {
        // given
        // - v8 is all ok
        setup_test();
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, v8::ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // given
        // - empty string
        let value: v8::Local<'_, Value> = v8::String::new(scope, "").unwrap().into();
        // when
        // - try to convert
        let result = try_as_i32(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert_eq!(0, result.expect("Expected to be able to convert"));

        // given
        // - negative string
        let value: v8::Local<'_, Value> = v8::String::new(scope, "-10").unwrap().into();
        // when
        // - try to convert
        let result = try_as_i32(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert_eq!(-10, result.expect("Expected to be able to convert"));

        // given
        // - integer value
        let value: v8::Local<'_, Value> = v8::String::new(scope, "123").unwrap().into();
        // when
        // - try to convert
        let result = try_as_i32(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert_eq!(123, result.expect("Expected to be able to convert"));

        // given
        // - float value
        let value: v8::Local<'_, Value> = v8::String::new(scope, "123.789").unwrap().into();
        // when
        // - try to convert
        let result = try_as_i32(&value, scope);
        // then
        // - expect to be able to convert and result in true
        assert_eq!(123, result.expect("Expected to be able to convert"));
    }

    #[test]
    fn test_try_i32_from_undefined_null() {
        // given
        // - v8 is all ok
        setup_test();
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, v8::ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // given
        // - undefined value
        let value: v8::Local<'_, Value> = v8::undefined(scope).into();
        // when
        // - try to convert
        let result = try_as_i32(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert_eq!(0, result.expect("Expected to be able to convert"));

        // given
        // - null value
        let value: v8::Local<'_, Value> = v8::null(scope).into();
        // when
        // - try to convert
        let result = try_as_i32(&value, scope);
        // then
        // - expect to be able to convert and result in true
        assert_eq!(0, result.expect("Expected to be able to convert"));
    }

    #[test]
    fn test_try_u32_from_string() {
        // given
        // - v8 is all ok
        setup_test();
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, v8::ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // given
        // - empty string
        let value: v8::Local<'_, Value> = v8::String::new(scope, "").unwrap().into();
        // when
        // - try to convert
        let result = try_as_u32(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert_eq!(0, result.expect("Expected to be able to convert"));

        // given
        // - negative string
        let value: v8::Local<'_, Value> = v8::String::new(scope, "-10").unwrap().into();
        // when
        // - try to convert
        let result = try_as_u32(&value, scope);
        // then
        // - expect to be able to convert and result in false
        result.expect_err("Expected to NOT be able to convert");

        // given
        // - integer value
        let value: v8::Local<'_, Value> = v8::String::new(scope, "123").unwrap().into();
        // when
        // - try to convert
        let result = try_as_u32(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert_eq!(123, result.expect("Expected to be able to convert"));

        // given
        // - float value
        let value: v8::Local<'_, Value> = v8::String::new(scope, "123.789").unwrap().into();
        // when
        // - try to convert
        let result = try_as_u32(&value, scope);
        // then
        // - expect to be able to convert and result in true
        result.expect_err("Expected to NOT be able to convert");
    }

    #[test]
    fn test_try_u32_from_undefined_null() {
        // given
        // - v8 is all ok
        setup_test();
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, v8::ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // given
        // - undefined value
        let value: v8::Local<'_, Value> = v8::undefined(scope).into();
        // when
        // - try to convert
        let result = try_as_u32(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert_eq!(0, result.expect("Expected to be able to convert"));

        // given
        // - null value
        let value: v8::Local<'_, Value> = v8::null(scope).into();
        // when
        // - try to convert
        let result = try_as_u32(&value, scope);
        // then
        // - expect to be able to convert and result in true
        assert_eq!(0, result.expect("Expected to be able to convert"));
    }

    #[test]
    fn test_try_i8_from_string() {
        // given
        // - v8 is all ok
        setup_test();
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, v8::ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // given
        // - out of range
        let value: v8::Local<'_, Value> = v8::String::new(scope, "-10").unwrap().into();
        // when
        // - try to convert
        let result = try_as_i8(&value, scope);
        // then
        // - expect to be able to convert and result in false
        assert_eq!(-10_i8, result.expect("Expected to be able to convert"));

        // given
        // - out of range
        let value: v8::Local<'_, Value> = v8::String::new(scope, "1024").unwrap().into();
        // when
        // - try to convert
        let result = try_as_i8(&value, scope);
        // then
        // - expect to be able to convert and result in false
        result.expect_err("Expected to NOT be able to convert");
    }
}
