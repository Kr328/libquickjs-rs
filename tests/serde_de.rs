#![cfg(feature = "serde")]

use std::collections::HashMap;

use libquickjs::{EvalFlags, Runtime, serde::from_value};
use serde::Deserialize;

#[test]
fn test_deserialize_object() {
    let rt = Runtime::new();

    let ctx = rt.new_context();
    let obj = ctx
        .eval_global(None, r#"({ a: 1, b: 2, c: { d: 3 }, e: 42 })"#, "test.js", EvalFlags::STRICT)
        .unwrap();

    #[derive(Deserialize)]
    struct NestedObject {
        d: i32,
    }

    #[derive(Deserialize)]
    struct Object {
        a: i32,
        b: i32,
        c: NestedObject,
    }

    let obj: Object = from_value(&ctx, &obj).unwrap();

    assert_eq!(obj.a, 1);
    assert_eq!(obj.b, 2);
    assert_eq!(obj.c.d, 3);
}

#[test]
fn test_deserialize_primitives() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    // Test string
    let str_val = ctx
        .eval_global(None, r#"("hello world")"#, "test.js", EvalFlags::STRICT)
        .unwrap();
    let str_result: String = from_value(&ctx, &str_val).unwrap();
    assert_eq!(str_result, "hello world");

    // Test number
    let num_val = ctx.eval_global(None, "(42)", "test.js", EvalFlags::STRICT).unwrap();
    let num_result: i32 = from_value(&ctx, &num_val).unwrap();
    assert_eq!(num_result, 42);

    // Test float
    let float_val = ctx.eval_global(None, "(Math.PI)", "test.js", EvalFlags::STRICT).unwrap();
    let float_result: f64 = from_value(&ctx, &float_val).unwrap();
    assert!((float_result - std::f64::consts::PI).abs() < 0.00001);

    // Test boolean
    let bool_val = ctx.eval_global(None, "(true)", "test.js", EvalFlags::STRICT).unwrap();
    let bool_result: bool = from_value(&ctx, &bool_val).unwrap();
    assert_eq!(bool_result, true);

    // Test null
    let null_val = ctx.eval_global(None, "(null)", "test.js", EvalFlags::STRICT).unwrap();
    let null_result: Option<i32> = from_value(&ctx, &null_val).unwrap();
    assert_eq!(null_result, None);

    // Test undefined
    let undefined_val = ctx.eval_global(None, "(undefined)", "test.js", EvalFlags::STRICT).unwrap();
    let undefined_result: Option<i32> = from_value(&ctx, &undefined_val).unwrap();
    assert_eq!(undefined_result, None);
}

#[test]
fn test_deserialize_arrays() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    // Test simple array
    let array_val = ctx
        .eval_global(None, "([1, 2, 3, 4, 5])", "test.js", EvalFlags::STRICT)
        .unwrap();
    let array_result: Vec<i32> = from_value(&ctx, &array_val).unwrap();
    assert_eq!(array_result, vec![1, 2, 3, 4, 5]);

    // Test mixed type array
    let mixed_val = ctx
        .eval_global(None, r#"([1, "two", 3, false, [1, 2]])"#, "test.js", EvalFlags::STRICT)
        .unwrap();
    #[derive(Deserialize, Debug, PartialEq)]
    enum MixedValue {
        Number(i32),
        String(String),
        Boolean(bool),
        Array(Vec<i32>),
    }
    let mixed_result: Vec<MixedValue> = from_value(&ctx, &mixed_val).unwrap();
    assert_eq!(mixed_result.len(), 5);

    assert_eq!(mixed_result[0], MixedValue::Number(1));
    assert_eq!(mixed_result[1], MixedValue::String("two".to_string()));
    assert_eq!(mixed_result[2], MixedValue::Number(3));
    assert_eq!(mixed_result[3], MixedValue::Boolean(false));
    assert_eq!(mixed_result[4], MixedValue::Array(vec![1, 2]));
}

#[test]
fn test_deserialize_enums() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    // Test tagged enum with rename
    let enum_val = ctx
        .eval_global(None, r#"({ type: "success", value: 42 })"#, "test.js", EvalFlags::STRICT)
        .unwrap();
    #[derive(Deserialize, Debug, PartialEq)]
    #[serde(tag = "type")]
    enum Result {
        #[serde(rename = "success")]
        Success { value: i32 },
        #[serde(rename = "error")]
        Error { message: String },
    }
    let enum_result: Result = from_value(&ctx, &enum_val).unwrap();
    assert_eq!(enum_result, Result::Success { value: 42 });
}

#[test]
fn test_deserialize_tuples() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    // Test tuple
    let tuple_val = ctx
        .eval_global(None, r#"([1, "hello", true])"#, "test.js", EvalFlags::STRICT)
        .unwrap();
    let tuple_result: (i32, String, bool) = from_value(&ctx, &tuple_val).unwrap();
    assert_eq!(tuple_result, (1, "hello".to_string(), true));

    // Test nested tuples
    let nested_tuple_val = ctx
        .eval_global(None, r#"([[1, 2], [3, 4]])"#, "test.js", EvalFlags::STRICT)
        .unwrap();
    let nested_tuple_result: Vec<(i32, i32)> = from_value(&ctx, &nested_tuple_val).unwrap();
    assert_eq!(nested_tuple_result, vec![(1, 2), (3, 4)]);
}

#[test]
fn test_deserialize_complex_nested() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    let complex_val = ctx
        .eval_global(
            None,
            r#"({
                id: 1,
                name: "Product",
                price: 29.99,
                tags: ["electronics", "gadget"],
                inStock: true,
                variants: [
                    { color: "black", size: "M", stock: 10 },
                    { color: "white", size: "L", stock: 5 }
                ],
                metadata: null
            })"#,
            "test.js",
            EvalFlags::STRICT,
        )
        .unwrap();

    #[derive(Deserialize, Debug, PartialEq)]
    struct ProductVariant {
        color: String,
        size: String,
        stock: i32,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct Product {
        id: i32,
        name: String,
        price: f64,
        tags: Vec<String>,
        in_stock: bool,
        variants: Vec<ProductVariant>,
        metadata: Option<i32>,
        area: Option<i32>,
    }

    let complex_result: Product = from_value(&ctx, &complex_val).unwrap();
    assert_eq!(complex_result.id, 1);
    assert_eq!(complex_result.name, "Product");
    assert!((complex_result.price - 29.99).abs() < 0.001);
    assert_eq!(complex_result.tags, vec!["electronics", "gadget"]);
    assert_eq!(complex_result.in_stock, true);
    assert_eq!(complex_result.variants.len(), 2);
    assert_eq!(complex_result.metadata, None);
    assert_eq!(complex_result.area, None);
}

#[test]
fn test_deserialize_with_serde_attributes() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    let attr_val = ctx
        .eval_global(
            None,
            r#"({
                user_name: "john_doe",
                user_email: "john@example.com",
                profile_data: {
                    age: 30,
                    country: "USA"
                },
                active: true,
                roles: ["user", "admin"]
            })"#,
            "test.js",
            EvalFlags::STRICT,
        )
        .unwrap();

    #[derive(Deserialize, Debug, PartialEq)]
    struct UserProfile {
        age: i32,
        country: String,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct User {
        #[serde(rename = "user_name")]
        username: String,

        #[serde(rename = "user_email")]
        email: String,

        #[serde(rename = "profile_data")]
        profile: UserProfile,

        active: bool,

        #[serde(default)]
        roles: Vec<String>,
    }

    let user_result: User = from_value(&ctx, &attr_val).unwrap();
    assert_eq!(user_result.username, "john_doe");
    assert_eq!(user_result.email, "john@example.com");
    assert_eq!(user_result.profile.age, 30);
    assert_eq!(user_result.profile.country, "USA");
    assert_eq!(user_result.active, true);
    assert_eq!(user_result.roles, vec!["user", "admin"]);
}

#[test]
fn test_deserialize_map() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    let map_val = ctx
        .eval_global(None, r#"({ a: 1, b: 2, c: 3 })"#, "test.js", EvalFlags::STRICT)
        .unwrap();

    let map_result: HashMap<String, i32> = from_value(&ctx, &map_val).unwrap();
    assert_eq!(map_result.len(), 3);
    assert_eq!(map_result.get("a"), Some(&1));
    assert_eq!(map_result.get("b"), Some(&2));
    assert_eq!(map_result.get("c"), Some(&3));

    let seq_map_result: Vec<(String, i32)> = from_value(&ctx, &map_val).unwrap();
    assert_eq!(seq_map_result.len(), 3);
    assert_eq!(seq_map_result.get(0), Some(&("a".to_string(), 1)));
    assert_eq!(seq_map_result.get(1), Some(&("b".to_string(), 2)));
    assert_eq!(seq_map_result.get(2), Some(&("c".to_string(), 3)));
}
