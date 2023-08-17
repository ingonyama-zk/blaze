# BLAZE

 <div align="center">blaze is a Rust library for ZK acceleration on Xilinx FPGAs.</div>

![ingo_BlazeFire_5d](https://github.com/ingonyama-zk/blaze/assets/2446179/6460173b-02af-4023-b055-c8274a0cbc21)

**Disclaimer:** This release is aimed at extending the design and handling of HW, this design isn’t backward compatible with previous and F1 support is currently broken, will be fixed in short time.

:fire: More Ingonyama AWS F1 AFIs will be public soon, stay tuned!

# Read/Write to Device in Rust

This Rust package offers a basic AXI infrastructure and the ability to work with user logic through custom modules.
The custom modules provided are designed for [MSM](src/ingo_msm) and [Poseidon hash](src/ingo_hash) and allow for the loading of user logic onto an FPGA.

In addition to supporting our binary, this library allows work with custom builds using warpshell https://github.com/Quarky93/warpshell.

## New Design

The new design is based on the idea of how a pool connection works with a database. Accordingly, this level includes interaction with a specific connection, and all things concerning connection selection, multiple connections, as well as state machines should be performed by a management layer.

On the connection side, we can provide an API to retrieve any necessary data (including firewall status, current task number, etc.) for management. The main design components are described below.

### DriverClient

The [DriverClient](src/driver_client/) module is designed to establish a connection between the FPGA/AWS and a known type of card, such as the C1100 card. It does not possess any knowledge about primitives.

The [DriverClient](src/driver_client/) provides basic IO methods and can load a binary, as well as provide specific and debug information about current HW. For a specific card type, the [DriverConfig](src/driver_client/dclient.rs) remains the same and can be accessed using the `driver_client_cfg` function.

It is important to note that the high-level management layer determines which client and primitive should be used. The [DriverClient](src/driver_client/) can be overused in this process.

How to create a new connection:

```rust
let dclient = DriverClient::new(&id,
DriverConfig::driver_client_cfg(CardType::C1100));
```

### DriverPrimitive

To simplify the process of using different primitives, the [DriverPrimitiveClient](src/driver_client/) was created. It is a wrapper around a [DriverClient](src/driver_client/) connection and includes the necessary configuration data for the primitive, an implementation of a common trait called [DriverPrimitiveClient](src/driver_client/), and public and private methods that are only valid for that primitive.

The configuration (e.g. for msm there are addresses space and curve description) for each primitive is provided based on the type of primitive, so there is no need to configure this manually on the high-level manager layer.

To create a new primitive instance for MSM, for example, one would use the following code:

```rust
let dclient = DriverClient::new(&id, DriverConfig::driver_client_cfg(CardType::C1100));
let driver = MSMClient::new(
    MSMInit {
        mem_type: PointMemoryType::DMA,
        is_precompute: true,
        curve: Curve::BLS381,
    },
    dclient,
);
```

The [DriverPrimitiveClient](src/driver_client/) is a trait that includes the basic functions of interaction with HW regarding calculations on a particular primitive. It can work with any type of data, whether it is a basic type or a tuple. The trait includes functions for initialization, setting input data, waiting for results, and getting results.

For data encapsulation, methods specific to each primitive can be divided into public (mainly methods for retrieving data from a particular offset) and private (methods for recording data or retrieving specific data for internal calculations).

### General Example of usage

We will refer to any type of primitive as `DriverPrimitiveClient` to show generality.

```rust
 let dclient = DriverClient::new(&id, DriverConfig::driver_client_cfg(CardType::C1100));
 let driver = DriverPrimitiveClient::new(dpc_type, dclient);

 let _ = driver.initialize(dpc_param);
 let _ = driver.set_data(dpc_input);
 driver.wait_result();
 let dpc_res = driver.result(None).unwrap().unwrap();
```

## MSM (Multi Scalar Multiplication) Module

This module supports three curves (BLS12_377, BLS12_381, BN254) and two types of point storage on HW: DMA and HBM.

This function sets data for compute MSM and has three different cases depending on the input parameters.

1. DMA only mode: - Addres for point in [`MSMConfig`].

```rust
MSMInput = {
    points: Some(points),
    scalars,
    nof_elements: msm_size,
    hbm_point_addr: None,
}
```

2. HBM mode set points to HBM and scalars by DMA: points will be loaded on hbm at address `hbm_addr` with an `offset`.

```rust
MSMInput = {
    points: Some(points),
    scalars,
    nof_elements: msm_size,
    hbm_point_addr: Some(hbm_addr, offset),
}
```

3. HBM mode set only scalars: points were loaded in previous iteretion on HBM.

```rust
MSMInput = {
    points: None,
    scalars,
    nof_elements: msm_size,
    hbm_point_addr: Some(hbm_addr, offset),
}
```

## NTT (Number Theoretic Transform) Module

This module implements the calculation of NTT of size `2^27`. To use it, the input byte vector of elements must be specified. Each element must be represented in little-endian. The result will be a similar byte vector.

It is worth noting that the data transfer process is slightly different from other modules. The following is an example of how to use NTT. More details can be found here: [LINK TO BLOG]

```rust
let dclient = DriverClient::new(&id, DriverConfig::driver_client_cfg(CardType::C1100));
let driver = NTTClient::new(NTT::Ntt, dclient);
let buf_host = 0;
let buf_kernel = 0;
driver.set_data(NTTInput {
    buf_host,
    data: in_vec,
})?;
driver.driver_client.initialize_cms()?;
driver.driver_client.reset_sensor_data()?;

driver.initialize(NttInit {})?;
driver.start_process(Some(buf_kernel))?;
driver.wait_result()?;
let res = driver.result(Some(buf_kernel))?.unwrap();
```

## Poseidon Module

## Running tests and benchmark

### MSM (Multi Scalar Multiplication) tests

To run tests for the MSM primitive, use the following command:

```

RUST_LOG=<LEVEL_LOG> cargo test -- <TEST_FILE> -- <TEST_NAME>
```

Also, different tests can require additional parameters:
`ID` `FILENAME`, and `MSM_SIZE`.

Replace `<LEVEL_LOG>` with the desired log level (e.g. info, debug). Set `FILENAME` with the path to the binary
file and `ID` with the number of the FPGA slot.
Also, it's possible to set up a number of points in MSM in the `MSM_SIZE` variable.

If the values of `ID` and `MSM_SIZE` are not provided, they will be defaulted to `ID=0` and `MSM_SIZE=8192`.

### NTT tests

To run tests for the NTT primitive, use the following command:

```

INFNAME=<INPUT_VEC_FILE> OUTFNAME=<REFERENCE_OUT_VEC> RUST_LOG=<LEVEL_LOG> cargo test -- integration_ntt
```

Also, different tests can require additional parameters:
`ID` `INFNAME`, and `OUTFNAME`.

Replace `<LEVEL_LOG>` with the desired log level (e.g. info, debug). Set `INFNAME` with the path to the input vector in little-endian byte format. Since we are testing correctness, set the path to the file with which you want to compare the result for the `OUTFNAME` variable. It should also be a little-endian byte vector
file and `ID` with the number of the FPGA slot.

If the value of `ID` is not provided, they will be defaulted to `ID=0`.

### NTT benchmark

Benchmarks for NTT are located in the benches directory, it's worth clarifying that there is no correctness check inside the benchmark - for that use the tests.

To run bench for the NTT primitive, use the following command:

```

INFNAME=<INPUT_VEC_FILE> RUST_LOG=<LEVEL_LOG> cargo bench
```

Also, bench can require additional parameters: `ID` and `INFNAME`. Set `INFNAME` with the path to the input vector in little-endian byte format.

If the value of `ID` is not provided, they will be defaulted to `ID=0`.

### Poseidon tests
