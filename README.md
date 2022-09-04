# cloud-ZK
##  A toolkit for developing ZKP acceleration in the cloud

Follow the instructions below in order to use AWS EC2 F1 FPGA instance in your code to get accelerated MSM. 
#### Note: Before continuing, please prepare an AFI according to our [guide](/A_Step-by-step_guide_for_building_an_AWS_AFI.MD). Alternatively, you can use [Ingonyama AFI](/A_Step-by-step_guide_for_building_an_AWS_AFI.MD#load-the-afi). 


<!-- Install the MSM Client -->
## Install the MSM Client


* Install and run the MSM client on the F1 instance :


* Clone ingonyama-zk/cloud-zk client design github:

      NOTE: the Ingonyama client code is written in Rust.

Instructions to install Rust can be <a href="https://www.rust-lang.org/tools/install">found here</a>. 

* Create a dir for the Ingonyama client in your project

```sh 
mkdir -p  <ingonyama-zk-client-folder-name>
```

* Clone the Ingonyama client

```sh
$ cd <ingonyama-zk-client-folder-name>
$ git clone https://github.com/ingonyama-zk/cloud-zk.git  (TBD)
$ cd <ingonyama-zk-client-folder-name>/rust-rw-device/
```

<!--Install the Linux xdma drivers-->
## Install the linux xdma drivers

* Go to xdma dir and run make

```sh
$ cd <ingonyama-zk-client-folder-name>/xdma
$ make
```
* To install drivers, run the following as root

```sh
$ sudo make install
$ sudo modprobe xdma
```
* Allow driver access

```sh
sudo chmod 77 /dev/xdma0_user
sudo chmod 77 /dev/xdma0_c2h_0
sudo chmod 77 /dev/xdma0_c2h_1
sudo chmod 77 /dev/xdma0_h2c_0
```
Once successful, drivers are 
<!--MSM client functionality-->
## MSM client functionality

In order to run the MSM core on a custom input, one can use rust-rw-device as follows, using the function ''msm_calc'' in rw_msm_to_dram.rs file.

### msm_calc_biguint(points: &Vec<BigUint>, scalars: &Vec<BigUint>, size: usize) -> ([BigUint; 3],Duration,u8)
This function receives 3 parameters:
1) points: Vec\<BigUint>
2) scalars: Vec\<BigUint>
3) size: \<usize>

### msm_calc_u32(points: &Vec<u32>, scalars: &Vec<u32>, size: usize) -> ([Vec<u32> ;3],Duration,u8)
This function receives 3 parameters:
1) points: Vec\<u32>
2) scalars: Vec\<u32>
3) size: \<usize>
This function is built to run with bytestreams of points scalars, all data should be represented as little endian.

The MSM we want to compute is with points given in affine coordinates: x_1, y_1,...,x_n, y_n, and scalars s_1,...,s_n.
* The points input will be a 2n size vector that contains the values: [(x_1,y_1),...,(x_n,y_n)] (i.e., the points one after the other).
* The scalar vector will be an n size vector containing [s_1,...,s_n].

Each BigUint element in these vectors is expected to be an unsigned big integer of size at most 48 bytes (1152 bits).
The output of the function is a vector containing the result in projective coordinates and therefore contains a 48-byte BigUint vector with 3 elements (i.e., (x,y,z) of the resulting point).

the functions return single point in Projective representation and task duration -> (x, y, z, duration).

<!--Using the MSM client-->
## Using the MSM client

Add to your (rust) code the appropriate “use directive”:
```rust 
use rust_rw_device::rw_msm_to_dram;
```

Calling the MSM function:
Call the function `msm_calc_biguint(&points, &scalars, size)` or `msm_calc_u32(&points, &scalars, size)` in rw_msm_to_dram.rs file, when the MSM calculation is needed.
 
<!-- License -->
## License 
The code is released under the GNU General Public License v3.0 license. See <a href="https://github.com/ingonyama-zk/cloud-zk/blob/main/LICENSE.md">LICENSE.md</a> for more information.
 
 
 

 
 

