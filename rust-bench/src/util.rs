use ark_ec::{AffineCurve, ProjectiveCurve};
use ark_ff::PrimeField;
use ark_std::UniformRand;
use num_bigint::BigUint;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub fn generate_points_scalars<G: AffineCurve>(len: usize) -> (Vec<G>, Vec<G::ScalarField>) {
    let mut rng = ChaCha20Rng::from_entropy();

    let points = <G::Projective as ProjectiveCurve>::batch_normalization_into_affine(
        &(0..len)
            .map(|_| G::Projective::rand(&mut rng))
            .collect::<Vec<_>>(),
    );

    let scalars = (0..len)
        .map(|_| G::ScalarField::rand(&mut rng))
        .collect::<Vec<_>>();

    (points, scalars)
}

pub fn generate_points_scalars_big_uint<G: AffineCurve>(n_points: i32) -> (Vec<BigUint>, Vec<BigUint>) {
    let (points, scalars) = generate_points_scalars::<rust_rw_device::curve::G1Affine>(1usize << n_points);

    let points_as_big_int = points.into_iter()
        .map(|point| [point.y.into_repr().into(), point.x.into_repr().into()])
        .flatten()
        .collect::<Vec<BigUint>>();

    let scalar_as_big_int = scalars.into_iter()
        .map(|scalar| scalar.into_repr().into())
        .collect::<Vec<BigUint>>();

    (points_as_big_int, scalar_as_big_int)
}