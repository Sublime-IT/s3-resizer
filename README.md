# Automatic S3 - Image Resizer

This project helps setting up the dependencies for automatically generating multiple image versions in S3. When new files are placed in your selected bucket, they will be generated to a set of sizes (specified by the project), a cloudfront function will ensure that parameters to the file such as (https://my.cloudfront.distro/image.png?width=500) will resolve into the correctly, already resized image.

If a width parameter is specified, but the size is not in the SIZES list then it will just default back to the standard image.

**BEFORE U BEGIN**: Please note that this S3 lambda function is invoked by an S3 create event, so if you modify the code please keep recursion in mind. After deploying always check the function, and be ready to press the "Throttle" button if unexpected recursion occurs. 

**Make sure the function exits instantly, if the uploaded file has a unique trademark of the resized file names!**. 

## Setup / Dependencies (to run locally)

TBD

## Running the project

TBD

## Setting up the project in AWS for a given bucket

TBD
