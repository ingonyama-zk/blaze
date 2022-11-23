# Read/Write to Device in Rust

This (relatively short) rust code provides a rust implementation of [AWS XDMA in C/C++](https://github.com/aws/aws-fpga/blob/master/sdk/linux_kernel_drivers/xdma/README.md#quick-example).

## Test read/write

To test the read/write operations on a new machine do the following:
* Clone ``cloud-msm`` repository and ``cd`` to ``rust-rw-device`` folder. 
* Run ``cargo build`` (it will show some warnings that can be ignored for now). 
* Run ``./target/debug/rust-rw-device <channel> <address_in_decimal>`` command. For example, if we work with the channel ``/dev/xdma0_user`` and offset ``0x00F0_0000`` (= 15728640 in decimal), we can run: ``./target/debug/rust-rw-device /dev/xdma0_user 15728640``.

The result in cmd should be:
```
-------------------
Working with channel ./try and address 15728640
Writing: 123456789101112131415161718
Writing: 123456789
Writing: 1
Read: 123456789101112131415161718
Read: 123456789
Read: 1
-------------------
Working with channel ./try and address 15728640
Writing: 15
Read: 15
Written number 15 in binary 1111
Mask with 0b1000 8
Mask with 0b0100 4
Mask with 0b1010 10
Mask with 0b1100 12
```