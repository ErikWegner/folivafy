#!/bin/bash

rm -rf generated
mkdir generated
docker run --rm \
    -v "${PWD}/generated:/generated" \
    -v "${PWD}/openapi.yml:/openapi.yml:ro" \
    -v "${PWD}/../openapi-generator/modules/openapi-generator/src/main/resources/rust-server:/templates:ro" \
    --user $(id -u):$(id -g) \
    openapitools/openapi-generator-cli:latest generate \
    -i /openapi.yml \
    -g rust-server \
    --template-dir /templates \
    --package-name openapi \
    --additional-properties=preferUnsignedInt=true \
    -o /generated
echo "disable_all_formatting = true" > generated/rustfmt.toml
