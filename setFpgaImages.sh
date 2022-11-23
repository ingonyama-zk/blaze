# clear both fpgas of old images
# sudo fpga-clear-local-image -S 0
# sudo fpga-clear-local-image -S 1

<<'###BLOCK-COMMENT'
New G1 250MHz build where padding is not needed. Input order is still [y, x] where x and y is 256b
{
    "FpgaImages": [
        {
            "FpgaImageId": "afi-05ff6a417ce1f8f76",
            "FpgaImageGlobalId": "agfi-01e758858844860d9",
            "Name": "rapidsnark-g1-f250",
            "State": {
                "Code": "pending"
            },
            "CreateTime": "2022-11-01T02:03:14.000Z",
            "UpdateTime": "2022-11-01T02:03:14.000Z",
            "OwnerId": "983370650745",
            "Tags": [],
            "Public": false,
            "DataRetentionSupport": false
        }
    ]
}
###BLOCK-COMMENT

# fpga @ slot 0 should be used for G1
sudo fpga-load-local-image -S 0 -I agfi-01e758858844860d9 -H

<<'###BLOCK-COMMENT'
New BN254 G2 187MHz
{
    "FpgaImages": [
        {
            "FpgaImageId": "afi-035b98142a8f20acf",
            "FpgaImageGlobalId": "agfi-05c523a947b48ff0a",
            "Name": "rapidsnark-g2-f187",
            "ShellVersion": "0x04261818",
            "PciId": {
                "DeviceId": "0xf001",
                "VendorId": "0x1d0f",
                "SubsystemId": "0x1d51",
                "SubsystemVendorId": "0xfedd"
            },
            "State": {
                "Code": "available"
            },
            "CreateTime": "2022-11-01T12:29:03.000Z",
            "UpdateTime": "2022-11-01T14:09:15.000Z",
            "OwnerId": "983370650745",
            "Tags": [],
            "Public": false,
            "DataRetentionSupport": false
        }
    ]
}
###BLOCK-COMMENT
# fpga @ slot 1 should be used for G2
sudo fpga-load-local-image -S 1 -I agfi-05c523a947b48ff0a -a 187