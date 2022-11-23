cd rust-rw-device
export TEST_NPOW=10
sudo fpga-load-local-image -S 0 -I agfi-00b931a86bfddf91d -H
echo "... device BLS12-377 G1 $TEST_NPOW debug"
cargo run --features=bls12-377 --no-default-features
echo "... device BLS12-377 G1 $TEST_NPOW release"
cargo run --features=bls12-377 --no-default-features --release

cd ../rust-bench;
export TEST_NPOW=12; 
echo "... test BLS12-377 G1 $TEST_NPOW"
cargo test --features=bls12-377 --no-default-features --release

echo "... load fpga images for BN254 G1 G2"
cd ..; ./setFpgaImages.sh

cd rust-rw-device
export TEST_NPOW=12
echo "... device BN254 G1 $TEST_NPOW debug"
cargo run 
echo "... device BN254 G1 $TEST_NPOW release"
cargo run --release

cd ../rust-rw-device-g2
export TEST_NPOW=15
echo "... device BN254 G2 $TEST_NPOW release"
cargo run --release
 
cd ../rust-bench;
export TEST_NPOW=12; 
echo "... test BN254 G1 $TEST_NPOW release"
cargo test --release
