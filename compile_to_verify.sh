#!/bin/bash -e

CIRCUIT_NAME="plonky2"

cargo test -r --package plonky2_circom_verifier --lib verifier::tests::test_recursive_verifier --no-fail-fast -- -Z unstable-options --show-output
cp ./circom/test/data/proof.json ./circom/e2e_tests/input.json
cd circom/e2e_tests
circom ../circuits/${CIRCUIT_NAME}.circom --r1cs --sym --wasm -c
cd ${CIRCUIT_NAME}_cpp
make -j
cd ../
./${CIRCUIT_NAME}_cpp/${CIRCUIT_NAME} input.json witness.wtns
zkutil setup -c ${CIRCUIT_NAME}.r1cs
zkutil prove -c ${CIRCUIT_NAME}.r1cs -w witness.wtns
zkutil generate-verifier -v ./hardhat/contracts/Verifier.sol
zkutil verify
