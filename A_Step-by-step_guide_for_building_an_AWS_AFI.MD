# A Step-by-step guide for building an AWS AFI
<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#Overview">Overview</a></li>
    <li>
      <a href="#Development-Flow">Development Flow</a></li>
    <li>
      <a href="#Development-Environments">Development Environments</a></li>
    <li>
      <a href="#Hardware-Development-Kit-(HDK)">Hardware Development Kit (HDK)</a></li>
    <li>
      <a href="#Runtime-Tools-(SDK)">Runtime Tools (SDK)</a></li>
    <li>
      <a href="#MSM-AFI">MSM AFI</a></li>
    <li>
      <a href="#Ingonyama-MSM">Ingonyama MSM</a></li>

  </ol>
</details>


<!-- Overview -->
## Overview
The purpose of this document is to describe the procedures needed for creating:
AWS AFI MSM acceleration image, using a special FPGA implementation for MSM. 
MSM client that uses the FPGA acceleration 


<a href="https://github.com/aws/aws-fpga">AWS EC2 FPGA Development Kit</a> is the Official repository of the AWS EC2 FPGA Hardware and Software Development Kit 

<!-- Development Flow -->
## Development Flow

* Creating the AFI
<p align="left">The Ingonyama HW team is developing the MSM FPGA design (also called CL - Custom logic), and then creating the Amazon FPGA Image (AFI) ready to easily deploy to an F1 instance. AFIs are Amazon reusable, shareable images and can be deployed in a scalable and secure way.
Please note that Ingonyama provides a ready to use AFI which is updated from time to time.
You may create and use your own AFI as described in this document. </p>

* Creating the MSM client
<p align="left">The Ingonyama SW team is developing a MSM client program to run MSM computations. The client compares and shows the efficiency of the FPGA implementation over CPU or GPU implementations.</p>

<!-- Development Environments -->
## Development Environments
<p>The <a href="https://aws.amazon.com/marketplace/pp/B06VVYBLZZ">FPGA Developer AMI</a> from the AWS marketplace is used for developing the FPGA Designs. It includes the tools needed for developing FPGA Designs to run on AWS F1.</p>
<p>The <a href="https://aws.amazon.com/marketplace/pp/B06VVYBLZZ">FPGA Developer AMI</a> contains:
Hardware Development Kit (HDK)
Runtime Tools (SDK)
</p>

<!-- Hardware Development Kit (HDK) -->
## Hardware Development Kit (HDK)

<p>The <a href="https://github.com/aws/aws-fpga/blob/master/hdk/README.md">HDK directory</a> contains documentation, examples, simulation, build and AFI creation scripts to start building Amazon FPGA Images (AFI).</p>
The HDK can be installed on any on-premises server or an EC2 instance. 
The developer kit is not required if you plan to use a pre-built AFI shared from another developer.

### AWS Partitions
With Amazon EC2 FPGA instances, each FPGA is divided into two partitions:
* Shell (SH) – AWS platform logic implementing the FPGA external peripherals, PCIe, DRAM, and Interrupts.
* Custom Logic (CL) – Custom acceleration logic created by an FPGA Developer, in our case, the FPGA MSM implementation.
<p>At the end of the development process, combining the Shell and CL creates an Amazon FPGA Image (AFI) that can be loaded onto the Amazon EC2 FPGA Instances.</p>
 
<!-- Runtime Tools (SDK) -->
## Runtime Tools (SDK)
The <a href="https://github.com/aws/aws-fpga/blob/master/sdk/README.md">SDK directory</a> includes the runtime environment required to run on EC2 FPGA instances (f1.2xlarge, f1.8xlarge). It includes the drivers and tools to manage the AFIs that are loaded on the FPGA instance. The SDK isn't required during the AFI development process; it is only required once an AFI is loaded onto an EC2 FPGA instance. The following SDK resources are provided:
* Linux Kernel Drivers - The developer kit includes three drivers:
  * <a href="https://github.com/aws/aws-fpga/blob/master/sdk/linux_kernel_drivers/xdma/README.md">XDMA Driver</a> - DMA interface to/from HDK accelerators.
* <a href="https://github.com/aws/aws-fpga/blob/master/sdk/userspace/fpga_libs">FPGA Libraries</a> - APIs used by C/C++ host applications.
* <a href="https://github.com/aws/aws-fpga/blob/master/sdk/userspace/fpga_mgmt_tools/README.md">FPGA Management</a> Tools - AFI management APIs for runtime loading/clearing FPGA image, gathering metrics and debug interface on the F1 instance.


<!-- MSM AFI -->
## MSM AFI

<!-- Getting Started -->
### Getting Started


* <a href="https://aws.amazon.com/free/">Setup an AWS Account</a>


* Sign into your <a href="https://aws.amazon.com/console/">AWS Console</a>


### Launch the AMI instance
Launch an instance using the <a href="https://aws.amazon.com/marketplace/pp/B06VVYBLZZ">FPGA Developer AMI</a> which comes pre-installed with Vivado and the required licenses. Given the large size of the FPGA used inside the AWS FPGA instances, the implementation tools require 32GiB Memory (e.g.: c4.4xlarge, m4.2xlarge, r4.xlarge, t2.2xlarge). c4.4xlarge and c4.8xlarge would provide the fastest execution time with 30 and 60GiB of memory respectively. 


### Setup AWS IAM permissions

Setup AWS IAM permissions for creating FPGA Images (CreateFpgaImage and DescribeFpgaImages). <a href="http://docs.aws.amazon.com/AWSEC2/latest/APIReference/ec2-api-permissions.html">EC2 API Permissions are described in more detail</a>. It is highly recommended that you validate your AWS IAM permissions prior to proceeding with this quick start. By calling the <a href="https://github.com/aws/aws-fpga/blob/master/hdk/docs/describe_fpga_images.md">DescribeFpgaImages API</a> you can check that your IAM permissions are correct.



### Setup AWS CLI and S3 Bucket to enable AFI creation.

Note that the AWS CLI is preinstalled on the FPGA Developer AMI.


#### Setup the AWS CLI.

* Create the AWS keys: 
   * AWS Access Key ID
   * AWS Secret Access Key
<p>Use the AWS console:  Identity and Access Management (IAM) -> Access management -> Users -> Click on your user name -> Click on security credentials tab -> Create access key. Once your keys have been created, download the access key file, as this is the only time where you can do so. Store it somewhere you can easily find it. If you lose this file, you will have to create a new access key.</p>
<p>Having the keys, configure the CLI:</p>

  ```
$ aws configure# to set your credentials (found in your
                    console.aws.amazon.com page) and default region
AWS Access Key ID [None]: AKIAIOSFODNN7EXAMPLE
AWS Secret Access Key [None]: wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
Default region name [None]: us-east-1
Default output format [None]: json
```

Use the aws-cli <a href="http://docs.aws.amazon.com/cli/latest/userguide/cli-command-line.html">region</a> command line argument to override the profile default region. Supported regions include: us-east-1, us-west-2, eu-west-1 and us-gov-west-1.




### Install the HDK


In the FPGA Developer AMI, define:

```
AWS_FPGA_REPO_DIR=/home/centos/src/project_data/aws-fpga
```

Clone aws-fpga github:

```
$ git clone https://github.com/aws/aws-fpga.git $AWS_FPGA_REPO_DIR
$ cd $AWS_FPGA_REPO_DIR
$ source hdk_setup.sh
```

<!-- Ingonyama MSM -->
## Ingonyama MSM

### Install the Ingonyama MSM design


Clone ingonyama-zk/cloud-zk
Create a dir for the Ingonyama design:

```sh
mkdir -p  <ingonyama-design-folder-name>
```

Clone the Ingonyama HW design

```sh
$ cd <ingonyama-design-folder-name>
$ git clone https://github.com/ingonyama-zk/FPGA_design (TBD)
```

Build the CL:

```sh
$ cd <ingonyama-design-folder-name>/../ingo_shell
$ make aws_ingo_msm
```

Create a bucket and folder for your tarball, then copy to S3

```
$ aws s3 mb s3://<bucket-name> --region <region> # Create an S3 bucket
                                           (choose a unique bucket name)
$ aws s3 mb s3://<bucket-name>/<dcp-folder-name>/   # Create folder for
                                                     your tarball files
$ cd <ingonyama-design-folder-name> 
$ cd ../ingo_shell/build/aws_ingo_msm/build/checkpoints/to_aws
$ aws s3 cp *.Developer_CL.tar \                # Upload the file to S3
  s3://<bucket-name>/<dcp-folder-name>/


 NOTE: The trailing '/' is required after <dcp-folder-name>
```

Create a folder on the s3 bucket for your log files

```
  $ aws s3 mb s3://<bucket-name>/<logs-folder-name>/  # Create a folder to
                                                         Keep your logs
  $ touch LOGS_FILES_GO_HERE.txt                     # Create a temp file
  $ aws s3 cp LOGS_FILES_GO_HERE.txt s3://<bucket-name>/<logs-folder-name>/
                                              #Which creates the folder on S3
```

        NOTE: The trailing '/' is required after <logs-folder-name


### Create the AFI


Start AFI creation:

```
$ aws ec2  create-fpga-image \
	     --region <region> \
           --name <afi-name> \
           --description <afi-description> \  
           --input-storage-location Bucket=<dcp-bucket-name>,Key=<path-to-tarball> \ 
           --logs-storage-location Bucket=<logs-bucket-name>,Key=<path-to-logs> \
    NOTE: <path-to-tarball> is <dcp-folder-name>/<tar-file-name>
          <path-to-logs> is <logs-folder-name>
```

The output of this command includes two identifiers that refer to your AFI:
* FPGA Image Identifier or AFI ID: this is the main ID used to manage your AFI through the AWS EC2 CLI commands and AWS SDK APIs. This ID is regional, i.e., if an AFI is copied across multiple regions, it will have a different unique AFI ID in each region. An example AFI ID is `afi-06d0ffc989feeea2a`.
* Global FPGA Image Identifier or AGFI ID: this is a global ID that is used to refer to an AFI from within an F1 instance. For example, to load or clear an AFI from an FPGA slot, you use the AGFI ID. Since the AGFI IDs is global (by design), it allows you to copy a combination of AFI/AMI to multiple regions, and they will work without requiring any extra setup. An example AGFI ID is `agfi-0f0e045f919413242`.
The <a href="https://github.com/aws/aws-fpga/blob/master/hdk/docs/describe_fpga_images.md">describe-fpga-images</a> API allows you to check the AFI state during the background AFI generation process.

You must provide the FPGA Image Identifier returned by `create-fpga-image` :

```
$ aws ec2 describe-fpga-images --fpga-image-ids afi-06d0ffc989feeea2a
```

You can use the <a href="https://github.com/aws/aws-fpga/blob/master/hdk/docs/wait_for_afi.md">wait_for_afi.py</a> script to wait for the AFI creation to complete and then optionally send an email with the results.

The AFI can only be loaded to an instance once the AFI generation completes and the AFI state is set to available:

```
   {
	"FpgaImages": [
            {
			    …

                "State": {
                    "Code": "available"
                },

			    …

                "FpgaImageId": "afi-06d0ffc989feeea2a",
			    …
            }
        ]
    }
```

After the AFI generation is complete, AWS will put the logs into the bucket location (s3://<bucket-name>/<logs-folder-name>) provided by the developer. The presence of these logs is an indication that the creation process is complete. Please look for either a “State” file indicating the state of the AFI (e.g., available or failed), or the Vivado logs detailing errors encountered during the creation process. For help with AFI creation issues, see <a href="https://github.com/aws/aws-fpga/blob/master/hdk/docs/create_fpga_image_error_codes.md">create-fpga-image error codes</a>

NOTE: Attempting to load the AFI immediately on an instance will result in an `Invalid AFI ID` error. Please wait until you confirm (using the The <a href="https://github.com/aws/aws-fpga/blob/master/hdk/docs/describe_fpga_images.md">describe-fpga-images</a> API) the AFI is created successfully.

NOTE: Attempting to load the AFI in a region different from where the AFI was created will result in an `Invalid AFI ID` error. AFIs need to be copied to regions. The <a href="https://github.com/aws/aws-fpga/blob/master/hdk/docs/copy_fpga_image.md">copy-fpga-image</a> API allows you to copy the AFI to other regions and avoid the time consuming `create-fpga-image` process. Copy will also preserve the source Global AFI ID and minimize region-specific changes to your instance code or scripts.


<!--Load The AFI-->
### Load the AFI
Ingonyama cloud-ZK Latest AFI details:

* Ingonyama AFI

```sh
   * `agfi-00b931a86bfddf91d`
   * `afi-08036a63bbd0a73d2`
   * `Region:  us-east-1`
```

     NOTE: from time to time we will update with released AFI, AGFI, and regions.

To follow the next steps, you have to launch an F1 instance. AWS recommends that you launch an instance with the latest Amazon Linux that has the FPGA Management tools included, or alternatively the FPGA Developer AMI with both the HDK and SDK.
You can now use the FPGA Management tools, from within your F1 instance, to load your AFI onto an FPGA on a specific slot. Make sure you clear any AFI you have previously loaded in your slot:

```
   $ sudo fpga-clear-local-image  -S 0
```

You can also invoke the fpga-describe-local-image command to learn about which AFI, if any, is loaded onto a particular slot. For example, if the slot is cleared (slot 0 in this example), you should get an output similar to the following:

```
   $ sudo fpga-describe-local-image -S 0 -H
```

```
    Type  FpgaImageSlot  FpgaImageId             StatusName    StatusCode   ErrorName    ErrorCode   ShVersion
    AFI          0       none                    cleared           1        ok               0       <shell_version>
    Type  FpgaImageSlot  VendorId    DeviceId    DBDF
    AFIDEVICE    0       0x1d0f      0x1042      0000:00:0f.0
```


<p>If fpga-describe-local-image API call returns a status 'Busy', the FPGA is still performing the previous operation in the background. Please wait until the status is 'Cleared' as above.</p>

Now, let us try loading your AFI to FPGA `slot 0`:

```
   $ sudo fpga-load-local-image -S 0 -I agfi-0fcf87119b8e97bf3
```

     NOTE: The FPGA Management tools use the AGFI ID (not the AFI ID).

Now, you can verify that the AFI was loaded properly. The output shows the FPGA in the “loaded” state after the FPGA image "load" operation. The "-R" option performs a PCI device remove and rescan in order to expose the unique AFI Vendor and Device Id.

```
   $ sudo fpga-describe-local-image -S 0 -R -H
    Type  FpgaImageSlot  FpgaImageId             StatusName    StatusCode   ErrorName    ErrorCode   ShVersion
    AFI          0       agfi-0fcf87119b8e97bf3  loaded            0        ok               0       0x04261818
    Type  FpgaImageSlot  VendorId    DeviceId    DBDF
    AFIDEVICE    0       0x1d0f      0xf000      0000:00:1d.0
```
 
 
 

 
 

