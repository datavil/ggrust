//! Arrow -> Column conversion for every supported input type.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use arrow_array::types::{Int8Type, Int32Type};
use arrow_array::{
    ArrayRef, BooleanArray, DictionaryArray, Float32Array, Float64Array, Int64Array,
    LargeStringArray, NullArray, RecordBatch, StringArray, StringViewArray,
    TimestampMillisecondArray, UInt8Array,
};
use arrow_schema::{Field, Schema};
use ggrust_core::Error;
use ggrust_core::data::{Column, DataFrame};

fn frame_of(columns: Vec<(&str, ArrayRef)>) -> DataFrame {
    let batch = RecordBatch::try_from_iter(columns).unwrap();
    DataFrame::new(batch.schema(), vec![batch]).unwrap()
}

fn continuous(column: Column) -> Vec<Option<f64>> {
    let Column::Continuous(column) = column else {
        panic!("expected continuous")
    };
    (0..column.len()).map(|row| column.get(row)).collect()
}

fn discrete(column: Column) -> (Vec<String>, Vec<Option<String>>) {
    let Column::Discrete(column) = column else {
        panic!("expected discrete")
    };
    let labels = (0..column.len())
        .map(|row| column.label(row).map(str::to_owned))
        .collect();
    (column.levels().to_vec(), labels)
}

#[test]
fn floats_pass_through_without_copying() {
    let values: ArrayRef = Arc::new(Float64Array::from(vec![Some(1.5), None, Some(f64::NAN)]));
    let frame = frame_of(vec![("x", values.clone())]);
    let Column::Continuous(column) = frame.column("x").unwrap() else {
        panic!("expected continuous")
    };
    assert_eq!(column.get(0), Some(1.5));
    assert_eq!(column.get(1), None);
    assert!(column.get(2).unwrap().is_nan());
    assert_eq!(column.null_count(), 1);
    let original = values.as_any().downcast_ref::<Float64Array>().unwrap();
    assert_eq!(
        column.values().as_ptr(),
        original.values().as_ptr(),
        "zero copy"
    );
}

#[test]
fn integers_and_float32_become_f64() {
    let frame = frame_of(vec![
        (
            "int",
            Arc::new(Int64Array::from(vec![Some(3), None])) as ArrayRef,
        ),
        ("uint", Arc::new(UInt8Array::from(vec![7, 8]))),
        ("float32", Arc::new(Float32Array::from(vec![0.5, 2.0]))),
    ]);
    assert_eq!(continuous(frame.column("int").unwrap()), [Some(3.0), None]);
    assert_eq!(
        continuous(frame.column("uint").unwrap()),
        [Some(7.0), Some(8.0)]
    );
    assert_eq!(
        continuous(frame.column("float32").unwrap()),
        [Some(0.5), Some(2.0)]
    );
}

#[test]
fn strings_keep_first_appearance_order() {
    let frame = frame_of(vec![
        (
            "utf8",
            Arc::new(StringArray::from(vec![
                Some("T"),
                Some("B"),
                None,
                Some("T"),
                Some("NK"),
            ])) as ArrayRef,
        ),
        (
            "large",
            Arc::new(LargeStringArray::from(vec!["b", "a", "b", "a", "a"])),
        ),
        (
            "view",
            Arc::new(StringViewArray::from(vec!["y", "x", "y", "y", "x"])),
        ),
    ]);
    let (levels, labels) = discrete(frame.column("utf8").unwrap());
    assert_eq!(levels, ["T", "B", "NK"]);
    assert_eq!(labels[2], None);
    assert_eq!(labels[3].as_deref(), Some("T"));
    assert_eq!(discrete(frame.column("large").unwrap()).0, ["b", "a"]);
    assert_eq!(discrete(frame.column("view").unwrap()).0, ["y", "x"]);
}

#[test]
fn booleans_are_discrete() {
    let frame = frame_of(vec![(
        "flag",
        Arc::new(BooleanArray::from(vec![Some(true), None, Some(false)])) as ArrayRef,
    )]);
    let (levels, labels) = discrete(frame.column("flag").unwrap());
    assert_eq!(levels, ["true", "false"]);
    assert_eq!(labels, [Some("true".into()), None, Some("false".into())]);
}

#[test]
fn dictionaries_keep_order_and_drop_unused_levels() {
    // Levels: T, unused, NK, B. Rows use B, T, null, NK, B.
    let dictionary: DictionaryArray<Int8Type> = DictionaryArray::new(
        vec![Some(3), Some(0), None, Some(2), Some(3)].into(),
        Arc::new(StringArray::from(vec!["T", "unused", "NK", "B"])),
    );
    let frame = frame_of(vec![("cluster", Arc::new(dictionary) as ArrayRef)]);
    let (levels, labels) = discrete(frame.column("cluster").unwrap());
    assert_eq!(levels, ["T", "NK", "B"]);
    assert_eq!(
        labels,
        [
            Some("B".into()),
            Some("T".into()),
            None,
            Some("NK".into()),
            Some("B".into())
        ]
    );
}

#[test]
fn dictionary_null_and_duplicate_values_are_handled() {
    let dictionary: DictionaryArray<Int32Type> = DictionaryArray::new(
        vec![0, 1, 2, 0].into(),
        Arc::new(StringArray::from(vec![Some("a"), None, Some("a")])),
    );
    let frame = frame_of(vec![("c", Arc::new(dictionary) as ArrayRef)]);
    let (levels, labels) = discrete(frame.column("c").unwrap());
    assert_eq!(levels, ["a"]);
    assert_eq!(
        labels,
        [Some("a".into()), None, Some("a".into()), Some("a".into())]
    );
}

#[test]
fn all_null_column_is_missing_continuous() {
    let frame = frame_of(vec![("empty", Arc::new(NullArray::new(2)) as ArrayRef)]);
    assert_eq!(continuous(frame.column("empty").unwrap()), [None, None]);
}

#[test]
fn chunks_are_concatenated_per_column() {
    let schema = Arc::new(Schema::new(vec![
        Field::new("x", arrow_schema::DataType::Float64, true),
        Field::new("c", arrow_schema::DataType::Utf8, true),
    ]));
    let batch = |x: Vec<f64>, c: Vec<&str>| {
        RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Float64Array::from(x)),
                Arc::new(StringArray::from(c)),
            ],
        )
        .unwrap()
    };
    let frame = DataFrame::new(
        schema.clone(),
        vec![
            batch(vec![1.0], vec!["b"]),
            batch(vec![2.0, 3.0], vec!["a", "b"]),
        ],
    )
    .unwrap();
    assert_eq!(frame.row_count(), 3);
    assert_eq!(
        continuous(frame.column("x").unwrap()),
        [Some(1.0), Some(2.0), Some(3.0)]
    );
    assert_eq!(discrete(frame.column("c").unwrap()).0, ["b", "a"]);
}

#[test]
fn zero_batches_give_empty_columns() {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "x",
        arrow_schema::DataType::Int32,
        true,
    )]));
    let frame = DataFrame::new(schema, vec![]).unwrap();
    assert_eq!(frame.row_count(), 0);
    assert!(frame.column("x").unwrap().is_empty());
}

#[test]
fn missing_column_lists_available_names() {
    let frame = frame_of(vec![(
        "UMAP1",
        Arc::new(Float64Array::from(vec![1.0])) as ArrayRef,
    )]);
    let error = frame.column("UMAP2").unwrap_err();
    assert!(matches!(error, Error::MissingColumn { .. }));
    assert_eq!(
        error.to_string(),
        "column 'UMAP2' not found in data; available columns: UMAP1"
    );
}

#[test]
fn unsupported_type_names_column_and_type() {
    let frame = frame_of(vec![(
        "time",
        Arc::new(TimestampMillisecondArray::from(vec![0])) as ArrayRef,
    )]);
    let error = frame.column("time").unwrap_err().to_string();
    assert!(
        error.starts_with("column 'time' has unsupported type Timestamp"),
        "{error}"
    );
}

#[test]
fn mismatched_batch_schema_is_rejected() {
    let batch = RecordBatch::try_from_iter(vec![(
        "x",
        Arc::new(Float64Array::from(vec![1.0])) as ArrayRef,
    )])
    .unwrap();
    let other = Arc::new(Schema::new(vec![Field::new(
        "y",
        arrow_schema::DataType::Float64,
        true,
    )]));
    assert!(matches!(
        DataFrame::new(other, vec![batch]),
        Err(Error::InvalidData(_))
    ));
}
