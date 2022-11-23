use std::str::FromStr;

use rust_rw_device::curve::G1Affine;
use ark_ff::BigInteger256;
use criterion::{Criterion, criterion_group, criterion_main};

use ingo_x::util;

fn bench_msm_ark_single(c: &mut Criterion) {
    let bench_npow = std::env::var("BENCH_NPOW").unwrap_or("23".to_string());
    let n_points = i32::from_str(&bench_npow).unwrap();

    let (points, scalars) = util::generate_points_scalars::<G1Affine>(1usize << n_points);
    let name = format!("2**{}", n_points);

    let mut group = c.benchmark_group("MSM_ARK_SINGLE");
    group.sample_size(20);

    group.bench_function(name, |b| {
        b.iter(|| {
            let _ = ingo_x::msm_ark(&points.as_slice(), unsafe {
                std::mem::transmute::<&[_], &[BigInteger256]>(
                    scalars.as_slice(),
                )
            });
        })
    });

    group.finish();
}

fn bench_msm_cloud_single(c: &mut Criterion) {
    let bench_npow = std::env::var("BENCH_NPOW").unwrap_or("23".to_string());
    let n_points = i32::from_str(&bench_npow).unwrap();

    let (points_as_big_int, scalar_as_big_int) = util::generate_points_scalars_big_uint::<G1Affine>(n_points);
    let name = format!("2**{}", n_points);

    let mut group = c.benchmark_group("MSM_CLOUD_SINGLE");
    group.sample_size(20);

    group.bench_function(name, |b| {
        b.iter(|| {
            let _ = ingo_x::msm_cloud::<G1Affine>(&points_as_big_int, &scalar_as_big_int);
        })
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = bench_msm_ark_single, bench_msm_cloud_single
}
criterion_main!(benches);