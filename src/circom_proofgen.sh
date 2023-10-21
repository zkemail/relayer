#!/bin/bash
set -e # Stop on error

if [ $# -ne 1 ]; then
    echo "Usage: $0 <nonce>"
    exit 1
fi

nonce=$1
zk_email_path="${MODAL_ZK_EMAIL_CIRCOM_PATH}"
HOME="${MODAL_ZK_EMAIL_CIRCOM_PATH}/../"
wallet_eml_dir_path=$MODAL_INCOMING_EML_PATH

if [ "$PROVER_LOCATION" = "local" ]; then
    zk_email_path=$LOCAL_ZK_EMAIL_CIRCOM_PATH
    HOME="${LOCAL_ZK_EMAIL_CIRCOM_PATH}/../"
    wallet_eml_dir_path=$LOCAL_INCOMING_EML_PATH
fi

prover_output_path="${wallet_eml_dir_path}/../proofs/"

wallet_eml_path="${wallet_eml_dir_path}/wallet_${nonce}.eml"
build_dir="${zk_email_path}/build/${CIRCUIT_NAME}"
input_wallet_path="${wallet_eml_dir_path}/input_${nonce}.json"
witness_path="${build_dir}/witness_${nonce}.wtns"
proof_path="${prover_output_path}/rapidsnark_proof_${nonce}.json"
public_path="${prover_output_path}/rapidsnark_public_${nonce}.json"

cd "${zk_email_path}"
echo "entered zk email path: ${zk_email_path}"

echo "npx tsx ${zk_email_path}/src/scripts/generate_input.ts --email_file='${wallet_eml_path}' --nonce='${nonce}'"
npx tsx "${zk_email_path}/src/scripts/generate_input.ts" --email_file="${wallet_eml_path}" --nonce="${nonce}" | tee /dev/stderr
status_inputgen=$?
echo "✓ Finished input gen! Status: ${status_inputgen}"

echo "node ${build_dir}/${CIRCUIT_NAME}_js/generate_witness.js ${build_dir}/${CIRCUIT_NAME}_js/${CIRCUIT_NAME}.wasm ${input_wallet_path} ${witness_path}"
node "${build_dir}/${CIRCUIT_NAME}_js/generate_witness.js" "${build_dir}/${CIRCUIT_NAME}_js/${CIRCUIT_NAME}.wasm" "${input_wallet_path}" "${witness_path}"  | tee /dev/stderr

status_jswitgen=$?
echo "✓ Finished witness gen with js! ${status_jswitgen}"

# TODO: Get C-based witness gen to work
# echo "/${build_dir}/${CIRCUIT_NAME}_cpp/${CIRCUIT_NAME} ${input_wallet_path} ${witness_path}"
# "/${build_dir}/${CIRCUIT_NAME}_cpp/${CIRCUIT_NAME}" "${input_wallet_path}" "${witness_path}"
# status_c_wit=$?

# echo "Finished C witness gen! Status: ${status_c_wit}"
# if [ $status_c_wit -ne 0 ]; then
#     echo "C based witness gen failed with status (might be on machine specs diff than compilation): ${status_c_wit}"
#     exit 1
# fi

if [ "$PROVER_LOCATION" = "local" ]; then
    # DEFAULT SNARKJS PROVER (SLOW)
    NODE_OPTIONS='--max-old-space-size=644000' ./node_modules/.bin/snarkjs groth16 prove "${build_dir}/${CIRCUIT_NAME}.zkey" "${witness_path}" "${proof_path}" "${public_path}"
    status_prover=$?
    echo "✓ Finished slow proofgen! Status: ${status_prover}"
else
    # RAPIDSNARK PROVER (10x FASTER)
    echo "ldd ${HOME}/rapidsnark/build/prover"
    ldd "${HOME}/rapidsnark/build/prover"
    status_lld=$?
    echo "✓ lld prover dependencies present! ${status_lld}"

    echo "${HOME}/rapidsnark/build/prover ${build_dir}/${CIRCUIT_NAME}.zkey ${witness_path} ${proof_path} ${public_path}"
    "${HOME}/rapidsnark/build/prover" "${build_dir}/${CIRCUIT_NAME}.zkey" "${witness_path}" "${proof_path}" "${public_path}"  | tee /dev/stderr
    status_prover=$?
    echo "✓ Finished rapid proofgen! Status: ${status_prover}"
fi



# TODO: Upgrade debug -> release and edit dockerfile to use release
echo "${HOME}/relayer/target/release/relayer chain false ${prover_output_path} ${nonce}"
"${HOME}/relayer/target/release/relayer" chain false "${prover_output_path}" "${nonce}" 2>&1 | tee /dev/stderr    
status_chain=$?
echo "✓ Finished send to chain! Status: ${status_chain}"

exit 0
