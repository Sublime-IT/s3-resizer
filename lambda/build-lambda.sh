#!/bin/bash

# Check if cross is installed
if ! command -v cross &> /dev/null
then
    echo "cross could not be found. Installing..."
    cargo install cross --git https://github.com/cross-rs/cross

    # Check if the installation was successful
    if ! command -v cross &> /dev/null
    then
        echo "Installation of cross failed. Exiting."
        exit 1
    fi
else
    echo "Found 'cross'"
fi

# Check if cargo lambda is installed
if ! command -v cargo-lambda &> /dev/null
then
    echo "cargo-lambda could not be found. Installing..."
    brew tap cargo-lambda/cargo-lambda
    brew install cargo-lambda

    # Check if the installation was successful
    if ! command -v cargo-lambda &> /dev/null
    then
        echo "Installation of cargo-lambda failed. Exiting."
        exit 1
    fi
else
    echo "Found 'cargo lambda'"
fi

# Cross compile lambda function
echo "Compiling lambda function..."
cargo lambda build --compiler cross --release

# Create a ZIP file with the binary
echo "Creating ZIP file with the binary..."
zip -j ./lambda.zip ./target/lambda/lambda_s3_thumbnail/bootstrap
