#!/bin/bash

CIRCUIT_NAME="email"
source "${ZK_EMAIL_CIRCOM_PATH}/../relayer/.env"

if [ $# -ne 1 ]; then
    echo "Usage: $0 <nonce>"
    exit 1
fi

nonce=$1
zk_email_path=$ZK_EMAIL_CIRCOM_PATH
HOME="${ZK_EMAIL_CIRCOM_PATH}/../"
wallet_eml_dir_path=$INCOMING_EML_PATH
prover_output_path="${wallet_eml_dir_path}/../proofs/"

wallet_eml_path="${wallet_eml_dir_path}/wallet_${nonce}.eml"
build_dir="${zk_email_path}/build/${CIRCUIT_NAME}"
input_wallet_path="${wallet_eml_dir_path}/input_${nonce}.json"
witness_path="${build_dir}/witness_${nonce}.wtns"
proof_path="${prover_output_path}/rapidsnark_proof_${nonce}.json"
public_path="${prover_output_path}/rapidsnark_public_${nonce}.json"

echo "npx tsx ${zk_email_path}/src/scripts/generate_input.ts --email_file=${wallet_eml_path} --nonce=${nonce}"
npx tsx "${zk_email_path}/src/scripts/generate_input.ts" --email_file="${wallet_eml_path}" --nonce="${nonce}"
status0=$?

echo "Finished input gen! Status: ${status0}"
if [ $status0 -ne 0 ]; then
    echo "generate_input.ts failed with status: ${status0}"
    exit 1
fi

"/${build_dir}/${CIRCUIT_NAME}_cpp/email" "${input_wallet_path}" "${witness_path}"
status_c_wit=$?

echo "Finished C witness gen! Status: ${status_c_wit}"
if [ $status_c_wit -ne 0 ]; then
    echo "generate_witness.js failed with status: ${status_c_wit}"
    exit 1
fi

# OLD: WASM witness gen
# node "${build_dir}/${CIRCUIT_NAME}_js/generate_witness.js" "${build_dir}/${CIRCUIT_NAME}_js/${CIRCUIT_NAME}.wasm" "${input_wallet_path}" "${witness_path}"
# status1=$?

# echo "Finished witness gen! Status: ${status1}"
# if [ $status1 -ne 0 ]; then
#     echo "generate_witness.js failed with status: ${status1}"
#     exit 1
# fi

"${HOME}/rapidsnark/build/prover" "${build_dir}/${CIRCUIT_NAME}.zkey" "${witness_path}" "${proof_path}" "${public_path}"
status2=$?

if [ $status2 -ne 0 ]; then
    echo "prover failed with status: ${status2}"
    exit 1
fi

echo "Finished proofgen! Status: ${status2}"

"cargo run --bin chain -- ${prover_output_path} ${nonce}"
status3=$?
if [ $status3 -ne 0 ]; then
    echo "Chain send failed with status: ${status3}"
    exit 1
fi

echo "Finished send to chain! Status: ${status3}"

exit 0

