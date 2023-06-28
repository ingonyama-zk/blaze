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

The [DriverClient](src/driver_client/) provides basic IO methods and can load a binary, as well as provide specific and debug information about current HW. For a specific card type, the [DriverConfig](src/driver_client/dclient.rs) remains the same and can be accessed using the `driver_client_c1100_cfg` function.

It is important to note that the high-level management layer determines which client and primitive should be used. The [DriverClient](src/driver_client/) can be overused in this process.

How to create a new connection:

```rust
let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
```

### DriverPrimitive

To simplify the process of using different primitives, the [DriverPrimitiveClient](src/driver_client/) was created. It is a wrapper around a [DriverClient](src/driver_client/) connection and includes the necessary configuration data for the primitive, an implementation of a common trait called [DriverPrimitiveClient](src/driver_client/), and public and private methods that are only valid for that primitive.

The configuration (e.g. for msm there are addresses space and curve description) for each primitive is provided based on the type of primitive, so there is no need to configure this manually on the high-level manager layer.

To create a new primitive instance for MSM, for example, one would use the following code:

```rust
let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
let driver = msm_api::MSMClient::new(
 msm_api::MSMInit {
 mem_type: msm_api::PointMemoryType::DMA,
 is_precompute: false,
 curve: msm_api::Curve::BLS381,
 },
 dclient,
);
```

The [DriverPrimitiveClient](src/driver_client/) is a trait that includes the basic functions of interaction with HW regarding calculations on a particular primitive. It can work with any type of data, whether it is a basic type or a tuple. The trait includes functions for initialization, setting input data, waiting for results, and getting results.

For data encapsulation, methods specific to each primitive can be divided into public (mainly methods for retrieving data from a particular offset) and private (methods for recording data or retrieving specific data for internal calculations).

### General Example of usage

We will refer to any type of primitive as `DriverPrimitiveClient` to show generality. And any abbreviation for a specific primitive will be replaced by `dpc` (e.g. `dpc_api` can be `msm_api` )

```rust
 let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
 let driver = dpc_api:: DriverPrimitiveClient::new(dpc_api::dpc_type, dclient);

 let params = driver.get_loaded_binary_parameters();
 let params_parse = dpc_api::DPCImageParametrs::parse_image_params(params[1]);

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

### Poseidon tests

```

```
