FROM rust:latest
ARG ZKEMAIL_BRANCH_NAME=anon_wallet
ARG RELAYER_BRANCH_NAME=modal_anon
ARG REFRESH_ZK_EMAIL=0
ARG REFRESH_RELAYER=0
ARG ZKEMAIL_COMMIT=dae73c1a03859f4eacd0fc565946a7095dd87e85

RUN apt-get update && apt-get upgrade -y 

# Update the package list and install necessary dependencies
RUN apt-get update && \
    apt install -y nodejs cmake build-essential pkg-config libssl-dev libgmp-dev libsodium-dev nasm awscli git tar

RUN npm install -g yarn npx

# Clone rapidsnark
RUN git clone https://github.com/Divide-By-0/rapidsnark /rapidsnark
WORKDIR /rapidsnark
RUN git submodule init
RUN git submodule update
RUN yarn install
RUN npx task createFieldSources
RUN npx task buildPistache
RUN npx task buildProver
RUN chmod +x /rapidsnark/build/prover

# Clone zk email repository at the latest commit and set it as the working directory
RUN git clone https://github.com/zkemail/zk-email-verify -b ${ZKEMAIL_BRANCH_NAME} /zk-email-verify
RUN mkdir /zk-email-verify/build && \
    cd /zk-email-verify/build && \
    curl -L https://s3.amazonaws.com/zkemail-zkey-chunks/${ZKEMAIL_COMMIT} | tar xz && \
    for file in *.tar.gz; do tar -xvf "$file"; done
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

