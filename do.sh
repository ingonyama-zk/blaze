./setFpgaImages.sh

cd rust-rw-device
export TEST_NPOW=12
#bn254
cargo run 
cargo run --release

cd ../rust-rw-device-g2
export TEST_NPOW=15
#bn254
cargo run --release
 