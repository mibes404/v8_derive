use v8::{ContextOptions, CreateParams};
use v8_derive::{
    macros::{FromValue, IntoValue},
    IntoValue, TryFromValue,
};

#[derive(FromValue, IntoValue)]
struct SimpleObject {
    yes_no: bool,
    name: String,
    age: i32,
    opt: Option<i32>,
}

#[derive(FromValue, IntoValue)]
struct ParentObject {
    nested: SimpleObject,
}

fn main() {
    // Create a Rust object
    let obj = SimpleObject {
        yes_no: true,
        name: "John".to_string(),
        age: 42,
        opt: Some(42),
    };

    // Nest the Rust object
    let parent_obj = ParentObject { nested: obj };

    // Create a Vec with the Rust object
    let short_vec = vec![parent_obj];

    // Initialize V8
    v8::V8::set_flags_from_string(
        "--no_freeze_flags_after_init --expose_gc --allow_natives_syntax --turbo_fast_api_calls",
    );
    v8::V8::initialize_platform(v8::new_unprotected_default_platform(0, false).make_shared());
    v8::V8::initialize();

    // Setup the V8 context
    let isolate = &mut v8::Isolate::new(CreateParams::default());
    let handle_scope = std::pin::pin!(v8::HandleScope::new(isolate));
    let handle_scope = &mut handle_scope.init();
    let context = v8::Context::new(handle_scope, ContextOptions::default());
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    // Convert the Rust object to a JS Value
    let js_obj = short_vec.into_value(scope);

    // Convert the JS Value back to a Rust object
    let rust_vec_obj = Vec::<ParentObject>::try_from_value(&js_obj, scope).unwrap();

    // Verify the Rust object
    let rust_parent_obj = rust_vec_obj.first().unwrap();
    let rust_obj = &rust_parent_obj.nested;
    assert!(rust_obj.yes_no);
    assert_eq!(rust_obj.name, "John");
    assert_eq!(rust_obj.age, 42);
    assert_eq!(rust_obj.opt, Some(42));
}
