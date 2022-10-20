use rust_rw_device::curve::G1Affine;
use ark_ff::BigInteger256;
use criterion::{Criterion, criterion_group, criterion_main};

use ingo_x::util;

fn bench_msm_ark(c: &mut Criterion) {
    let mut group = c.benchmark_group("MSM_ARK");
    group.sample_size(20);

    for n_points in 15..26 {
        let (points, scalars) = util::generate_points_scalars::<G1Affine>(1usize << n_points);

        let name = format!("2**{}", n_points);
        group.bench_function(name, |b| {
            b.iter(|| {
                let _ = ingo_x::msm_ark(&points.as_slice(), unsafe {
                    std::mem::transmute::<&[_], &[BigInteger256]>(
                        scalars.as_slice(),
                    )
                });
            })
        });
    }

    group.finish();
}

fn bench_msm_cloud(c: &mut Criterion) {
    let mut group = c.benchmark_group("MSM_CLOUD");
    group.sample_size(20);

    for n_points in 15..26 {
        let (points_as_big_int, scalar_as_big_int) = util::generate_points_scalars_big_uint::<G1Affine>(n_points);
        let name = format!("2**{}", n_points);

        group.bench_function(name, |b| {
            b.iter(|| {
                let _ = ingo_x::msm_cloud::<G1Affine>(&points_as_big_int, &scalar_as_big_int);
            })
        });
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = bench_msm_ark, bench_msm_cloud
}
criterion_main!(benches);