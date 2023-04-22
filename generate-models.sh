#!/bin/bash

rm -rf generated
mkdir generated
docker run --rm \
    -v "${PWD}/generated:/generated" \
    -v "${PWD}/openapi.yml:/openapi.yml:ro" \
    -v "${PWD}/../openapi-generator/modules/openapi-generator/src/main/resources/rust-server:/templates:ro" \
    --user $(id -u):$(id -g) \
    erikwegner/openapi-generator-cli:v6.5.0-rust-1 generate \
    -i /openapi.yml \
    -g rust-server \
    --template-dir /templates \
    --package-name openapi \
    --additional-properties=preferUnsignedInt=true \
    -o /generated
