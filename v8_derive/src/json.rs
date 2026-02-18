use crate::{
    errors::{Error, Result},
    IntoValue, TryFromValue,
};
use v8::{Local, PinScope, Value};

/// Convert a V8 Object to a JSON Value
///
/// # Errors
/// In case of conversion errors, or if the value is not supported, an error is returned.
pub(crate) fn v8_to_json_value(scope: &mut PinScope<'_, '_>, value: Local<Value>) -> Result<serde_json::Value> {
    match () {
        () if value.is_string() => {
            let value = String::try_from_value(&value, scope)?;
            Ok(serde_json::Value::String(value))
        }
        () if value.is_int32() => {
            let value = i32::try_from_value(&value, scope)?;
            Ok(serde_json::Value::from(value))
        }
        () if value.is_uint32() => {
            let value = u32::try_from_value(&value, scope)?;
            Ok(serde_json::Value::from(value))
        }
        () if value.is_big_int() => {
            let value = i64::try_from_value(&value, scope)?;
            Ok(serde_json::Value::from(value))
        }
        () if value.is_number() => {
            let value = f64::try_from_value(&value, scope)?;
            Ok(serde_json::Value::from(value))
        }
        () if value.is_boolean() => {
            let value = bool::try_from_value(&value, scope)?;
            Ok(serde_json::Value::from(value))
        }
        () if value.is_null() => Ok(serde_json::Value::Null),
        () if value.is_array() => v8_array_to_json(scope, value),
        () if value.is_object() => v8_object_to_json(scope, value),
        () => Err(Error::UnsupportedValueType),
    }
}

fn v8_object_to_json(scope: &mut PinScope<'_, '_>, value: Local<Value>) -> Result<serde_json::Value> {
    let Some(object) = value.to_object(scope) else {
        return Err(Error::ExpectedObject);
    };
    let Some(properties) = object.get_property_names(scope, v8::GetPropertyNamesArgs::default()) else {
        return Err(Error::FailedToGetPropertyNames);
    };
    let length = properties.length();
    let mut json_object = serde_json::Map::with_capacity(length as usize);
    for i in 0..length {
        let Some(key) = properties.get_index(scope, i) else {
            return Err(Error::ExpectedObject);
        };
        let key_str = String::try_from_value(&key, scope)?;
        let Some(value) = object.get(scope, key) else {
            return Err(Error::ExpectedObject);
        };
        let value = v8_to_json_value(scope, value)?;
        json_object.insert(key_str, value);
    }
    Ok(serde_json::Value::Object(json_object))
}

fn v8_array_to_json(scope: &mut PinScope<'_, '_>, value: Local<Value>) -> Result<serde_json::Value> {
    let Ok(array) = value.try_cast::<v8::Array>() else {
        return Err(Error::ExpectedArray);
    };
    let length = array.length();
    let mut json_array = Vec::with_capacity(length as usize);
    for i in 0..length {
        let item = match array.get_index(scope, i) {
            Some(item) => v8_to_json_value(scope, item)?,
            None => serde_json::Value::Null,
        };
        json_array.push(item);
    }
    Ok(json_array.into())
}

// Convert serde_json::Value to a V8 Object
pub(crate) fn json_to_v8<'s>(scope: &mut PinScope<'s, '_>, value: serde_json::Value) -> Local<'s, Value> {
    match value {
        serde_json::Value::Null => v8::null(scope).into(),
        serde_json::Value::Bool(b) => b.into_value(scope),
        serde_json::Value::Number(n) => {
            if let Some(n) = n.as_i64() {
                return n.into_value(scope);
            }
            if let Some(n) = n.as_f64() {
                return n.into_value(scope);
            }

            // todo: handle other number types; u64 is not supported in V8 so we return i64::MAX
            i64::MAX.into_value(scope)
        }
        serde_json::Value::String(s) => s.into_value(scope),
        serde_json::Value::Array(arr) => {
            let js_array = v8::Array::new(scope, i32::try_from(arr.len()).unwrap_or(i32::MAX));
            for (i, item) in arr.into_iter().enumerate() {
                let v8_value = json_to_v8(scope, item);
                js_array.set_index(scope, u32::try_from(i).unwrap_or(u32::MAX), v8_value);
            }
            js_array.into()
        }
        serde_json::Value::Object(obj) => {
            let js_object = v8::Object::new(scope);
            for (key, val) in obj {
                let v8_value = json_to_v8(scope, val);
                let v8_key = key.into_value(scope);
                js_object.set(scope, v8_key, v8_value);
            }
            js_object.into()
        }
    }
}
