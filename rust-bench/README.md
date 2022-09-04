# Usage

## Test

The tests will check that msm results from the [ark library](https://github.com/arkworks-rs/algebra/tree/master/ec/src/scalar_mul) 
and cloud-msm are identical for different pairs of vectors of size `2^23`.

```
cargo test
```

The size can be changed by adding environment variables `TEST_NPOW=X`.

Where `X` is the degree of two for the required size of the vectors.

```
TEST_NPOW=X cargo test
```

## Bench 
Generates a set of bench for vectors of size `2^15` to `2^26`.
```
cargo bench --bench msm
```

Generates a single bench for vectors of size `2^23`. 
```
cargo bench --bench msm_single
```
The size can be changed by adding environment variables `BENCH_NPOW=X`.
Where `X` is the degree of two for the required size of the vectors.
```
BENCH_NPOW=X cargo bench --bench msm_single
```
