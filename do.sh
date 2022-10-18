cd rust-rw-device
export TEST_NPOW=10
#bn254
sudo fpga-load-local-image -S 0 -I agfi-04bfc92ee8e9d78b2 -H 
cargo run --features=bn254 --no-default-features
cargo run --features=bn254 --no-default-features --release
#bls12-377
sudo fpga-load-local-image -S 0 -I agfi-00b931a86bfddf91d -H
cargo run
cargo run --release

export TEST_NPOW=15
#bn254
sudo fpga-load-local-image -S 0 -I agfi-04bfc92ee8e9d78b2 -H 
cargo run --features=bn254 --no-default-features --release
#bls12-377
sudo fpga-load-local-image -S 0 -I agfi-00b931a86bfddf91d -H
cargo run --release