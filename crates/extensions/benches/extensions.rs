//! Stable extension boundary benchmarks for comparing script backends.
//!
//! The pre-migration Rhai baseline was recorded with:
//! `cargo bench -p renrs-extensions --bench extensions -- --save-baseline rhai`
//!
//! After changing the backend, compare against it with:
//! `cargo bench -p renrs-extensions --bench extensions -- --baseline rhai`

use std::collections::BTreeMap;
use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use renrs_extensions::Extensions;
use renrs_syntax::syntax::Value;

const SCALAR: &str = "perform return(input * 2 + 1)\n";
const IDENTITY: &str = "perform return(input)\n";
const APPEND: &str = "perform return(push(input, 4096))\n";
const SUM: &str = r"
set total = 0
set index = 0
while index < len(input):
    set total = total + get(input, index)
    set index = index + 1
perform return(total)
";

fn sources(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(name, source)| ((*name).to_owned(), (*source).to_owned()))
        .collect()
}

fn extension(source: &str) -> Extensions {
    Extensions::new(&sources(&[("bench", source)])).expect("benchmark extension should compile")
}

fn list(size: i64) -> Value {
    Value::List(Arc::new((0..size).map(Value::Integer).collect()))
}

fn record(size: usize) -> Value {
    Value::Record(Arc::new(
        (0..size)
            .map(|index| {
                (
                    format!("field_{index:04}"),
                    Value::Integer(i64::try_from(index).expect("fixture size should fit in i64")),
                )
            })
            .collect(),
    ))
}

fn compile(criterion: &mut Criterion) {
    let scalar = sources(&[("scalar", SCALAR)]);
    let suite = sources(&[
        ("append", APPEND),
        ("identity", IDENTITY),
        ("scalar", SCALAR),
        ("sum", SUM),
    ]);
    let mut group = criterion.benchmark_group("extensions/compile");
    group.bench_function("scalar", |bencher| {
        bencher
            .iter(|| Extensions::new(black_box(&scalar)).expect("scalar extension should compile"));
    });
    group.bench_function("suite_4", |bencher| {
        bencher
            .iter(|| Extensions::new(black_box(&suite)).expect("extension suite should compile"));
    });
    group.finish();
}

fn invoke(criterion: &mut Criterion) {
    let scalar = extension(SCALAR);
    let scalar_input = Value::Integer(21);
    assert_eq!(
        scalar.invoke("bench", &scalar_input).unwrap(),
        Value::Integer(43)
    );

    let identity = extension(IDENTITY);
    let list_input = list(1024);
    let record_input = record(512);
    assert_eq!(identity.invoke("bench", &list_input).unwrap(), list_input);
    assert_eq!(
        identity.invoke("bench", &record_input).unwrap(),
        record_input
    );

    let append = extension(APPEND);
    let append_input = list(1024);
    let appended = append.invoke("bench", &append_input).unwrap();
    let Value::List(appended) = appended else {
        panic!("append benchmark should return a list");
    };
    assert_eq!(appended.len(), 1025);
    assert_eq!(appended.last(), Some(&Value::Integer(4096)));

    let sum = extension(SUM);
    let sum_input = list(256);
    assert_eq!(
        sum.invoke("bench", &sum_input).unwrap(),
        Value::Integer(32_640)
    );

    let mut group = criterion.benchmark_group("extensions/invoke");
    group.bench_function("scalar_arithmetic", |bencher| {
        bencher.iter(|| {
            black_box(
                scalar
                    .invoke("bench", black_box(&scalar_input))
                    .expect("scalar invocation should succeed"),
            )
        });
    });
    group.bench_function("list_identity_1024", |bencher| {
        bencher.iter(|| {
            black_box(
                identity
                    .invoke("bench", black_box(&list_input))
                    .expect("list invocation should succeed"),
            )
        });
    });
    group.bench_function("record_identity_512", |bencher| {
        bencher.iter(|| {
            black_box(
                identity
                    .invoke("bench", black_box(&record_input))
                    .expect("record invocation should succeed"),
            )
        });
    });
    group.bench_function("list_append_1024", |bencher| {
        bencher.iter(|| {
            black_box(
                append
                    .invoke("bench", black_box(&append_input))
                    .expect("append invocation should succeed"),
            )
        });
    });
    group.bench_function("sum_loop_256", |bencher| {
        bencher.iter(|| {
            black_box(
                sum.invoke("bench", black_box(&sum_input))
                    .expect("sum invocation should succeed"),
            )
        });
    });
    group.finish();
}

criterion_group!(benches, compile, invoke);
criterion_main!(benches);
