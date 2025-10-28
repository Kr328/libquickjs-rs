#![cfg(feature = "serde")]

use libquickjs::{EvalFlags, Runtime, Value, serde::to_value};
use serde::{Serialize, Serializer};

#[test]
fn test_serialize_object() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    #[derive(Serialize)]
    struct Nested {
        a: i32,
        b: String,
    }

    #[derive(Serialize)]
    struct Object {
        a: i32,
        b: String,
        c: Vec<i32>,
        d: Nested,
    }

    let obj = Object {
        a: 1,
        b: "hello".to_string(),
        c: vec![1, 2, 3],
        d: Nested {
            a: 2,
            b: "world".to_string(),
        },
    };

    let value = to_value(&ctx, &obj).expect("to value");
    assert!(matches!(&value, Value::Object(_)));

    ctx.set_property_str(&ctx.get_global_object(), "obj", value.clone())
        .expect("set property obj");

    let json = ctx
        .eval_global(None, "JSON.stringify(obj)", "test.js", EvalFlags::empty())
        .expect("eval stringify");
    let json = ctx.get_string(&json).expect("get string");
    assert_eq!(json.trim(), r#"{"a":1,"b":"hello","c":[1,2,3],"d":{"a":2,"b":"world"}}"#);

    let a = ctx.get_property_str(&value, "a").expect("get property a");
    assert_eq!(&a, &Value::Int32(1));
    let b = ctx.get_property_str(&value, "b").expect("get property b");
    assert_eq!(&*ctx.get_string(&b).unwrap(), "hello");
    let c = ctx.get_property_str(&value, "c").expect("get property c");
    assert!(ctx.is_array(&c));
    let d = ctx.get_property_str(&value, "d").expect("get property d");
    assert!(matches!(&d, Value::Object(_)));

    let c0 = ctx.get_property_uint32(&c, 0).expect("get property c0");
    assert_eq!(&c0, &Value::Int32(1));
    let c1 = ctx.get_property_uint32(&c, 1).expect("get property c1");
    assert_eq!(&c1, &Value::Int32(2));
    let c2 = ctx.get_property_uint32(&c, 2).expect("get property c2");
    assert_eq!(&c2, &Value::Int32(3));

    let da = ctx.get_property_str(&d, "a").expect("get property da");
    assert_eq!(&da, &Value::Int32(2));
    let db = ctx.get_property_str(&d, "b").expect("get property db");
    assert_eq!(&*ctx.get_string(&db).unwrap(), "world");
}

#[test]
fn test_serialize_primitives() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    // Test boolean
    let bool_value = to_value(&ctx, &true).expect("serialize bool");
    assert!(matches!(&bool_value, Value::Bool(true)));

    // Test integer
    let int_value = to_value(&ctx, &42).expect("serialize int");
    assert!(matches!(&int_value, Value::Int32(42)));

    // Test float
    let float_value = to_value(&ctx, &3.14).expect("serialize float");
    assert!(matches!(&float_value, Value::Float64(f) if (*f - 3.14).abs() < 0.00001));

    // Test string
    let str_value = to_value(&ctx, &"hello world").expect("serialize string");
    assert!(matches!(&str_value, Value::String(_)));
    assert_eq!(&*ctx.get_string(&str_value).unwrap(), "hello world");

    // Test unit (serializes to undefined)
    let unit_value = to_value(&ctx, &()).expect("serialize unit");
    assert!(matches!(&unit_value, Value::Undefined));

    // Test char
    let char_value = to_value(&ctx, &'A').expect("serialize char");
    assert!(matches!(&char_value, Value::String(_)));
    assert_eq!(&*ctx.get_string(&char_value).unwrap(), "A");
}

#[test]
fn test_serialize_arrays() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    // Test simple array
    let simple_array = vec![1, 2, 3, 4, 5];
    let array_value = to_value(&ctx, &simple_array).expect("serialize array");
    assert!(ctx.is_array(&array_value));

    // Verify array elements
    for (i, &expected) in simple_array.iter().enumerate() {
        let element = ctx.get_property_uint32(&array_value, i as u32).expect("get array element");
        assert_eq!(&element, &Value::Int32(expected));
    }

    // Test empty array
    let empty_array: Vec<i32> = vec![];
    let empty_array_value = to_value(&ctx, &empty_array).expect("serialize empty array");
    assert!(ctx.is_array(&empty_array_value));
    assert_eq!(ctx.get_length(&empty_array_value).expect("get array length"), 0);

    // Test nested array
    let nested_array = vec![vec![1, 2], vec![3, 4]];
    let nested_array_value = to_value(&ctx, &nested_array).expect("serialize nested array");
    assert!(ctx.is_array(&nested_array_value));

    // Verify nested array elements
    let sub_array1 = ctx.get_property_uint32(&nested_array_value, 0).expect("get sub array 1");
    let sub_array2 = ctx.get_property_uint32(&nested_array_value, 1).expect("get sub array 2");
    assert!(ctx.is_array(&sub_array1));
    assert!(ctx.is_array(&sub_array2));
    assert_eq!(&ctx.get_property_uint32(&sub_array1, 0).unwrap(), &Value::Int32(1));
    assert_eq!(&ctx.get_property_uint32(&sub_array2, 1).unwrap(), &Value::Int32(4));
}

#[test]
fn test_serialize_hashmap() {
    use std::collections::HashMap;

    let rt = Runtime::new();
    let ctx = rt.new_context();

    // Create and populate a HashMap
    let mut map = HashMap::new();
    map.insert("key1", "value1");
    map.insert("key2", "value2");
    map.insert("key3", "value3");

    // Serialize the HashMap
    let map_value = to_value(&ctx, &map).expect("serialize hashmap");
    assert!(matches!(&map_value, Value::Object(_)));

    // Verify object properties
    for (key, expected_value) in &map {
        let value = ctx.get_property_str(&map_value, key).expect("get property");
        assert_eq!(&*ctx.get_string(&value).unwrap(), *expected_value);
    }

    // Test empty HashMap
    let empty_map: HashMap<String, String> = HashMap::new();
    let empty_map_value = to_value(&ctx, &empty_map).expect("serialize empty hashmap");
    assert!(matches!(&empty_map_value, Value::Object(_)));
}

#[test]
fn test_serialize_option() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    // Test Some value
    let some_value = to_value(&ctx, &Some(42)).expect("serialize Some");
    assert_eq!(&some_value, &Value::Int32(42));

    // Test None value (serializes to null)
    let none_value = to_value(&ctx, &Option::<i32>::None).expect("serialize None");
    assert!(matches!(&none_value, Value::Null));

    // Test Some string
    let some_str_value = to_value(&ctx, &Some("hello")).expect("serialize Some string");
    assert!(matches!(&some_str_value, Value::String(_)));
    assert_eq!(&*ctx.get_string(&some_str_value).unwrap(), "hello");
}

#[test]
fn test_serialize_enum() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    // Define a test enum
    #[derive(Serialize)]
    enum TestEnum {
        Unit,
        Tuple(i32, String),
        Struct { field1: i32, field2: String },
    }

    // Test unit variant
    let unit_enum = to_value(&ctx, &TestEnum::Unit).expect("serialize unit enum");
    assert!(matches!(&unit_enum, Value::String(_)));
    assert_eq!(&*ctx.get_string(&unit_enum).unwrap(), "Unit");

    // Test tuple variant
    let tuple_enum = to_value(&ctx, &TestEnum::Tuple(42, "hello".to_string())).expect("serialize tuple enum");
    assert!(ctx.is_array(&tuple_enum));
    assert_eq!(&ctx.get_property_uint32(&tuple_enum, 0).unwrap(), &Value::Int32(42));
    assert_eq!(
        &*ctx.get_string(&ctx.get_property_uint32(&tuple_enum, 1).unwrap()).unwrap(),
        "hello"
    );

    // Test struct variant
    let struct_enum = to_value(
        &ctx,
        &TestEnum::Struct {
            field1: 42,
            field2: "hello".to_string(),
        },
    )
    .expect("serialize struct enum");
    assert!(matches!(&struct_enum, Value::Object(_)));
    assert_eq!(&ctx.get_property_str(&struct_enum, "field1").unwrap(), &Value::Int32(42));
    assert_eq!(
        &*ctx
            .get_string(&ctx.get_property_str(&struct_enum, "field2").unwrap())
            .unwrap(),
        "hello"
    );
}

#[test]
fn test_serialize_complex_nested() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    // Define complex nested structures
    #[derive(Serialize)]
    struct Inner {
        value: i32,
        options: Option<String>,
        items: Vec<f64>,
    }

    #[derive(Serialize)]
    struct Middle {
        name: String,
        inner: Inner,
        flags: Vec<bool>,
    }

    #[derive(Serialize)]
    struct Outer {
        id: u64,
        middle: Option<Middle>,
        data: Vec<Option<i32>>,
    }

    // Create a complex nested object
    let complex = Outer {
        id: 123456789,
        middle: Some(Middle {
            name: "test".to_string(),
            inner: Inner {
                value: 42,
                options: Some("optional".to_string()),
                items: vec![1.1, 2.2, 3.3],
            },
            flags: vec![true, false, true],
        }),
        data: vec![Some(1), None, Some(3)],
    };

    // Serialize the complex object
    let complex_value = to_value(&ctx, &complex).expect("serialize complex nested object");
    assert!(matches!(&complex_value, Value::Object(_)));

    // Verify properties
    assert_eq!(&ctx.get_property_str(&complex_value, "id").unwrap(), &Value::Int32(123456789));

    // Check middle object
    let middle = ctx.get_property_str(&complex_value, "middle").expect("get middle");
    assert!(matches!(&middle, Value::Object(_)));
    assert_eq!(
        &*ctx.get_string(&ctx.get_property_str(&middle, "name").unwrap()).unwrap(),
        "test"
    );

    // Check inner object
    let inner = ctx.get_property_str(&middle, "inner").expect("get inner");
    assert!(matches!(&inner, Value::Object(_)));
    assert_eq!(&ctx.get_property_str(&inner, "value").unwrap(), &Value::Int32(42));
}

#[test]
fn test_serialize_bytes() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    struct Bytes<'a> {
        pub bytes: &'a [u8],
    }

    impl<'a> Serialize for Bytes<'a> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_bytes(&self.bytes)
        }
    }

    // Test bytes serialization (serializes to ArrayBuffer)
    let bytes = vec![1u8, 2u8, 3u8, 4u8, 5u8];
    let bytes_value = to_value(&ctx, &Bytes { bytes: &bytes }).expect("serialize bytes");

    // Verify it's an ArrayBuffer
    assert!(ctx.is_array_buffer(&bytes_value));

    unsafe {
        // Verify the ArrayBuffer contents
        if let Ok(buffer) = ctx.get_array_buffer(&bytes_value) {
            assert_eq!(buffer, bytes);
        }
    }
}
