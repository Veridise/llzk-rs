use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use picus::{
    ident::Ident,
    vars::{VarKind, VarStr, Vars},
};
use std::hint::black_box;

#[derive(Hash, Eq, PartialEq, PartialOrd, Ord, Debug, Copy, Clone, Default)]
struct Key(usize, usize, usize);

impl VarKind for Key {
    fn is_input(&self) -> bool {
        false
    }

    fn get_input_no(&self) -> Option<usize> {
        None
    }

    fn is_output(&self) -> bool {
        false
    }

    fn get_output_no(&self) -> Option<usize> {
        None
    }

    fn is_temp(&self) -> bool {
        false
    }
}

impl From<Key> for VarStr {
    fn from(value: Key) -> Self {
        Ident::from(format!("var_{}_{}", value.0, value.1)).into()
    }
}

pub fn insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");

    for n in [1, 5, 10, 50, 100] {
        group.throughput(Throughput::Elements(n));
        group.bench_with_input(BenchmarkId::new("direct", n), &(n as usize), |b, i| {
            b.iter(|| {
                let mut vars = Vars::<Key>::default();
                for n in 0..(*i) {
                    vars.insert_with_value(
                        black_box(Key(1, n, 0)),
                        black_box(VarStr::try_from(format!("var_1_{n}")).unwrap()),
                    );
                }
            })
        });
        group.bench_with_input(BenchmarkId::new("equal", n), &(n as usize), |b, i| {
            b.iter(|| {
                let mut vars = Vars::<Key>::default();
                for _ in 0..(*i) {
                    vars.insert(black_box(Key(1, *i, 0)));
                }
            })
        });
        group.bench_with_input(BenchmarkId::new("different", n), &(n as usize), |b, i| {
            b.iter(|| {
                let mut vars = Vars::<Key>::default();
                for n in 0..(*i) {
                    vars.insert(black_box(Key(1, n, 0)));
                }
            })
        });

        group.bench_with_input(
            BenchmarkId::new("different same string", n),
            &(n as usize),
            |b, i| {
                b.iter(|| {
                    let mut vars = Vars::<Key>::default();
                    for n in 0..(*i) {
                        vars.insert(black_box(Key(1, *i, n)));
                    }
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, insert);
criterion_main!(benches);
