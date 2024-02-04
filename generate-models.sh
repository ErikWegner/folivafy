#!/bin/bash

rm -rf generated
mkdir generated
docker run --rm \
    -v "${PWD}/generated:/generated" \
    -v "${PWD}/openapi.yml:/openapi.yml:ro" \
    -v "${PWD}/openapi-generator-templates:/templates:ro" \
    --user $(id -u):$(id -g) \
    openapitools/openapi-generator-cli:v7.2.0 generate \
    -i /openapi.yml \
    -g rust-server \
    --template-dir /templates \
    --package-name openapi \
    --additional-properties=preferUnsignedInt=true \
    -o /generated
cp generated/src/models.rs src/models.rs
