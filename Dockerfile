FROM alpine/git as relayer_git
ARG RELAYER_BRANCH_NAME=modal_anon
RUN git clone --branch ${RELAYER_BRANCH_NAME} --single-branch https://github.com/zkemail/relayer /relayer
WORKDIR /relayer
RUN git rev-parse HEAD > relayer_latest_commit_hash

FROM alpine/git as zkemail_git
ARG ZKEMAIL_BRANCH_NAME=origin/anon_wallet
RUN git clone --branch ${ZKEMAIL_BRANCH_NAME} --single-branch https://github.com/zkemail/zk-email-verify /zk-email-verify
WORKDIR /zk-email-verify
RUN git rev-parse HEAD > zkemail_latest_commit_hash

ARG LATEST_RELAYER_COMMIT_HASH
ARG LATEST_ZKEMAIL_COMMIT_HASH

# Use the official Rust image as the base image
FROM rust:latest

# Install Node.js 16.x and Yarn
RUN curl -fsSL https://deb.nodesource.com/setup_16.x | bash - && \
    apt-get install -y nodejs && \
    curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | gpg --dearmor -o /usr/share/keyrings/yarn-archive-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/yarn-archive-keyring.gpg] https://dl.yarnpkg.com/debian/ stable main" > /etc/apt/sources.list.d/yarn.list && \
    apt-get update && \
    apt-get install -y yarn

# Update the package list and install necessary dependencies
RUN apt-get update && \
    apt install -y cmake build-essential pkg-config libssl-dev libgmp-dev libsodium-dev nasm

# Clone rapidsnark
RUN  git clone https://github.com/Divide-By-0/rapidsnark /rapidsnark
COPY ./rapidsnark/build /rapidsnark/build
WORKDIR /rapidsnark
RUN npm install
RUN git submodule init
RUN git submodule update
RUN chmod +x /rapidsnark/build/prover
RUN npx task createFieldSources
RUN npx task buildPistache
RUN npx task buildProver

# Copy the zkemail_latest_commit_hash files from the git stages
COPY --from=zkemail_git /zk-email-verify/zkemail_latest_commit_hash /zkemail_latest_commit_hash

# Clone zk email repository at the latest commit and set it as the working directory
RUN git clone https://github.com/zkemail/zk-email-verify -b anon_wallet /zk-email-verify
COPY ./zk-email-verify/build /zk-email-verify/build
WORKDIR /zk-email-verify
RUN yarn install

# Copy the relayer_latest_commit_hash files from the git stages
COPY --from=relayer_git /relayer/relayer_latest_commit_hash /relayer_latest_commit_hash

# Clone the relayer repository at the latest commit and set it as the working directory
RUN git clone --branch ${RELAYER_BRANCH_NAME} --single-branch https://github.com/zkemail/relayer /relayer \
    && echo "Going to check out latest relayer commit hash: ${LATEST_COMMIT_HASH}"

WORKDIR /relayer
RUN git checkout ${LATEST_COMMIT_HASH}

# Build for any AWS machine
RUN cargo build --target x86_64-unknown-linux-gnu
RUN cp /relayer/target/x86_64-unknown-linux-gnu/debug/relayer /relayer/target/debug/
RUN cargo build --target x86_64-unknown-linux-gnu --release
RUN cp /relayer/target/x86_64-unknown-linux-gnu/release/relayer /relayer/target/release/

# Make necessary files executable
RUN chmod +x /relayer/src/circom_proofgen.sh
