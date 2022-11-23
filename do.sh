./setFpgaImages.sh

cd rust-rw-device
export TEST_NPOW=12
#bn254
cargo run 
cargo run --release

cd ../rust-rw-device-G2
export TEST_NPOW=15
#bn254
cargo run --release

cd ../rust-bench;
#bn254
export TEST_NPOW=12; cargo test --release
export TEST_NPOW=3; cargo test --release
export TEST_NPOW=0; cargo test --release
 