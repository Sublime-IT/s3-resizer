# AWS - S3 Bucket - Image Optimizer

This project aims to make it easy to setup an S3 bucket with automatic image optimization. This creates a bucket, where any images uploaded to it will automatically be converted to WEBP and resized to a bunch of various sizes. It also creates a cloudfront distribution, and sets up a cloudfront function to automatically resolve "?width=NNN" parameters to the correct image. This approach aims to pre-generate all sizes, and not do it *on-the-fly*.

This means there is initially more compute and space required than doing it "on-the-fly", however; no users (even on low traffic pages) will experience longer loading times of images because they are always generated and ready to use.

When new files are placed in your selected bucket, they will be generated to a set of sizes (specified by the project), a cloudfront function will ensure that parameters to the file such as (https://my.cloudfront.distro/image.png?width=1920) will resolve into the resized image. With the default configuration, images will also be converted to webp - But it is easily modifable if this is not wanted (see releases for different configurations). You can also expand the code to include both original versions and webp versions, then change the cloudfront function to use an additional parameter (&format=webp).

If a width parameter is specified, but the size is not in the SIZES list then it will just default back to the standard image.

**BEFORE U BEGIN**: Please note that this S3 lambda function is invoked by an S3 create event, so if you modify the code please keep recursion in mind. After deploying always check the function, and be ready to press the "Throttle" button if unexpected recursion occurs. 

**Make sure the function exits instantly, if the uploaded file has a unique trademark of the resized file names!**. 

## Setup / Dependencies (to run locally)

TBD

## Running the project

TBD

## Setting up on AWS

### If you are creating a new S3 bucket from scratch

The easiest approach is creating a completely new bucket from scratch. 
Start by adjusting the size parameters in the file

> lambda/generate-sizes/src/main.rs

Set the array to the sizes you want to generate: (for example, if u want [ 128, 256, ..., 1920 ])

```RUST
const SIZES: [u32; n] = [ 128, 256, ..., 1920 ];
```

Now build the project 
(Note: If you don't want to build it manually, you can just download the lambda.zip file from the "Releases" tab. It has the standard configuration).

```bash
sudo chmod +x ./build-lambda.sh
./build-lambda.sh
```

After that update the cloudfront javascript to reflect the sizes too:

> cloudfront/rewrite-width-parameter-s3.js

```JS
const sizes = [ 128, 256, ..., 1920 ];
```

In AWS - If you don't already have a CloudFormation template bucket, then create one in S3 (for example: `cf-templates-l8fry6qzchzu-us-east-1`).

Next upload the following two files to the cloudformation template bucket:

 - cloudformation/create-base-stack.json
 - lambda/generate-sizes/lambda.zip

Go to `CloudFormation` and press `Create Stack`.

Choose "Amazon S3 Url", and fill in: (e.g. https://s3.us-east-1.amazonaws.com/cf-templates-l8fry6qzchzu-us-east-1/create-base-stack.json)

> `https://s3.{REGION}.amazonaws.com/{BUCKET_NAME}/create-base-stack.json`

Next fill in the name of the new bucket you want to create (in bucket name).

LambdaCodeS3Bucket is the bucket you uploaded the `lambda/generate-sizes/lambda.zip` file to.
LambdaCodeS3Key is the path of the bucket (e.g. if it's in root, it will just be `lambda.zip`).

Then create the cloudformation, and wait for it to complete.

When it is complete, go to lambda and select the new lambda function (for example `my-sample-test-s3-bucket-rust-lambda`). Then:

1. Click on the `Configuration` tab
2. Select `S3` as source
3. For the bucket, select the newly created bucket
4. Event-Types should be set to `All object create events`
5. Optionally, you can add a prefix/suffix if there is a certain sub-folder and/or image type that the generations should only happen to
6. Add the trigger

Next head over to the `CloudFront` service that was created. Go into functions, and create a new `cloudfront-js-2.0` function. Give it a name - This can be `rewrite-width-parameter-s3`

In the `Function code` paste the contents of `cloudfront/rewrite-width-parameter-s3.js` into it. Then save changes, and publish them.

Now go back into the CloudFront service and then:

1. Click on the `Behaviours` tab, and select the default behaviour. 
2. Press "Edit"
3. In the bottom, where it says `Viewer Request` choose "CloudFront Functions" and then "rewrite-width-parameter-s3".
4. Save changes

This is all setup you need. Try uploading an image to the new bucket, and then visit the cloudfront url:
 * https://my.cloudfront.url/test-image.jpg?width=128
 * https://my.cloudfront.url/test-image.jpg?width=1920

If the size is not a pre-defined size, it will fallback to the original size.
