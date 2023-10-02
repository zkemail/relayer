FROM rust:latest
ARG ZKEMAIL_BRANCH_NAME=anon_wallet
ARG RELAYER_BRANCH_NAME=modal_anon
ARG REFRESH_ZK_EMAIL=0
ARG REFRESH_RELAYER=0
ARG ZKEMAIL_COMMIT=e6592d86cb200d98d46db62d63404e7214a11569

RUN apt-get update && apt-get upgrade -y 

# Update the package list and install necessary dependencies
RUN apt-get update && \
    apt install -y nodejs npm cmake build-essential pkg-config libssl-dev libgmp-dev libsodium-dev nasm awscli git tar

RUN npm install -g yarn npx
 
# Copy rapidsnark build folder from Docker Hub
RUN docker pull aayushg0/rapidsnark:latest
RUN docker create --name rapidsnark aayushg0/rapidsnark:latest
RUN docker cp rapidsnark:/rapidsnark/build /rapidsnark/build
RUN docker rm -v rapidsnark
WORKDIR /rapidsnark/build
RUN chmod +x /rapidsnark/build/prover

# Clone zk email repository at the latest commit and set it as the working directory
RUN git clone https://github.com/zkemail/zk-email-verify -b ${ZKEMAIL_BRANCH_NAME} /zk-email-verify
RUN mkdir /zk-email-verify/build 
WORKDIR /zk-email-verify/build
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_nonchunked.zkey
RUN mkdir /zk-email-verify/build/wallet_js
RUN mkdir /zk-email-verify/build/wallet_cpp
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_js/generate_witness.js -o ./wallet_js/generate_witness.js
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_js/wallet.wasm -o ./wallet_js/wallet.wasm
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_js/wallet.wat -o ./wallet_js/wallet.wat
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_js/witness_calculator.js -o ./wallet_js/witness_calculator.js
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_cpp/calcwit.cpp -o ./wallet_cpp/calcwit.cpp
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_cpp/calcwit.hpp -o ./wallet_cpp/calcwit.hpp
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_cpp/circom.hpp -o ./wallet_cpp/circom.hpp
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_cpp/fr.asm -o ./wallet_cpp/fr.asm
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_cpp/fr.cpp -o ./wallet_cpp/fr.cpp
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_cpp/fr.hpp -o ./wallet_cpp/fr.hpp
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_cpp/main.cpp -o ./wallet_cpp/main.cpp
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_cpp/Makefile -o ./wallet_cpp/Makefile
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_cpp/wallet.cpp -o ./wallet_cpp/wallet.cpp
RUN curl -L https://zkemail-zkey-chunks.s3.amazonaws.com/${ZKEMAIL_COMMIT}/wallet_cpp/wallet.dat -o ./wallet_cpp/wallet.dat
WORKDIR /zk-email-verify

RUN yarn install

# Clone the relayer repository at the latest commit and set it as the working directory
RUN git clone --branch ${RELAYER_BRANCH_NAME} --single-branch https://github.com/zkemail/relayer /relayer
WORKDIR /relayer
RUN cargo build --target x86_64-unknown-linux-gnu
RUN cargo build --target x86_64-unknown-linux-gnu --release

# Build for any AWS machine
RUN cargo build --target x86_64-unknown-linux-gnu
RUN cp /relayer/target/x86_64-unknown-linux-gnu/debug/relayer /relayer/target/debug/
RUN cargo build --target x86_64-unknown-linux-gnu --release
RUN cp /relayer/target/x86_64-unknown-linux-gnu/release/relayer /relayer/target/release/

# Update repos to latest commits
RUN git pull
RUN cargo build --target x86_64-unknown-linux-gnu
RUN cp /relayer/target/x86_64-unknown-linux-gnu/debug/relayer /relayer/target/debug/

WORKDIR /zk-email-verify
RUN git pull
RUN yarn install
RUN git pull

# Make necessary files executable
RUN chmod +x /relayer/src/circom_proofgen.sh

