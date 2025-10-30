use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use picus::{
    ident::Ident,
    vars::{VarKind, VarStr, Vars},
};
use std::hint::black_box;

/// A key similar to the one used by the Halo2 frontend.
///
/// Hashing this struct is not as intense as hashing a string but is not as simple as hashing a
/// single machine word.
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

/// Benchmark for different insertion scenarios in the [`Vars`] environment.
///
/// Tests element by element insertion in the following scenarios:
///  1. Direct insertion on the environment without checking for name collisions. Serves as a baseline
///  of directly inserting on the underlying data structures.
///  2. Repeatedly inserting the same element, triggering cache hits.
///  3. Inserting different elements each time. Should be similar to the direct insertion plus a
///  check for uniqueness.
///  4. Inserting different elements that result in the same string representation. Forces the
///  uniquer to check several times for unique names.
///
/// For each scenario we progressively tests with a larger amount of inputs: 1, 5, 10, 50, 100.
/// This gives a decent enough sample size for checking performance and completing the benchmark in
/// a decent time even if its under the usual workload seen on production.
///
/// The most common scenarios are #2 and #3 so those are the ones that would need targeted
/// optimizations, if needed.
///
/// Note: The calls to [`black_box`] are for hinting the optimizer to leave the argument value alone.
/// Helps with avoiding the optimizer fudging with the code and screwing the results.
pub fn insert(c: &mut Criterion) {
    // We group the different tests together in a group to make the bechmarking framework show
    // their statistics together.
    let mut group = c.benchmark_group("insert");

    for n in [1, 5, 10, 50, 100] {
        group.throughput(Throughput::Elements(n));

        // Direct insertion without checking for name collissions
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

        // Repeatedly insert the same element, triggering cache hits
        group.bench_with_input(BenchmarkId::new("equal", n), &(n as usize), |b, i| {
            b.iter(|| {
                let mut vars = Vars::<Key>::default();
                for _ in 0..(*i) {
                    vars.insert(black_box(Key(1, *i, 0)));
                }
            })
        });

        // Insert different elements each time
        group.bench_with_input(BenchmarkId::new("different", n), &(n as usize), |b, i| {
            b.iter(|| {
                let mut vars = Vars::<Key>::default();
                for n in 0..(*i) {
                    vars.insert(black_box(Key(1, n, 0)));
                }
            })
        });

        // Insert different elements that result in the same string representation
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
