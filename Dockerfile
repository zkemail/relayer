FROM alpine/git as relayer_git
RUN git clone --branch ${RELAYER_BRANCH_NAME} --single-branch https://github.com/zkemail/relayer /relayer
WORKDIR /relayer
RUN git rev-parse HEAD > relayer_latest_commit_hash
RUN echo ${RELAYER_BRANCH_NAME} > relayer_branch_name

FROM alpine/git as zkemail_git
RUN git clone --branch ${ZKEMAIL_BRANCH_NAME} --single-branch https://github.com/zkemail/zk-email-verify /zk-email-verify
WORKDIR /zk-email-verify
RUN git rev-parse HEAD > zkemail_latest_commit_hash
RUN echo ${ZKEMAIL_BRANCH_NAME} > zkemail_branch_name

# Use the official Rust image as the base image
FROM rust:latest
ARG RELAYER_BRANCH_NAME=modal_anon
ARG ZKEMAIL_BRANCH_NAME=anon_wallet

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
COPY --from=zkemail_git /zk-email-verify/. /zk-email-verify-env/
# COPY --from=zkemail_git /zk-email-verify/zkemail_latest_commit_hash /zkemail_latest_commit_hash
# COPY --from=zkemail_git /zk-email-verify/zkemail_branch_name /zkemail_branch_name

# Clone zk email repository at the latest commit and set it as the working directory
RUN LATEST_ZKEMAIL_COMMIT_HASH=$(cat /zk-email-verify-env/zkemail_latest_commit_hash) && \
    ZKEMAIL_BRANCH_NAME=$(cat /zk-email-verify-env/zkemail_branch_name) && \
    git clone https://github.com/zkemail/zk-email-verify -b ${ZKEMAIL_BRANCH_NAME} /zk-email-verify && \
    echo "Going to check out latest zk email commit hash: ${LATEST_COMMIT_HASH}"

COPY ./zk-email-verify/build /zk-email-verify/build
WORKDIR /zk-email-verify
RUN yarn install

# Copy the relayer_latest_commit_hash files from the git stages
COPY --from=relayer_git /relayer/. /relayer-env/
# COPY --from=relayer_git /relayer/zrelayer_branch_name /relayer_branch_name
# COPY --from=relayer_git /relayer/relayer_latest_commit_hash /relayer_latest_commit_hash
ARG RELAYER_BRANCH_NAME

# Clone the relayer repository at the latest commit and set it as the working directory
RUN LATEST_RELAYER_COMMIT_HASH=$(cat /relayer-env/relayer_latest_commit_hash) && \
    RELAYER_BRANCH_NAME=$(cat /relayer-env/relayer_branch_name) && \
    git clone --branch ${RELAYER_BRANCH_NAME} --single-branch https://github.com/zkemail/relayer /relayer \
    && echo "Going to check out latest relayer commit hash: ${LATEST_RELAYER_COMMIT_HASH}"

WORKDIR /relayer
RUN git checkout ${LATEST_COMMIT_HASH}

# Build for any AWS machine
RUN cargo build --target x86_64-unknown-linux-gnu
RUN cp /relayer/target/x86_64-unknown-linux-gnu/debug/relayer /relayer/target/debug/
RUN cargo build --target x86_64-unknown-linux-gnu --release
RUN cp /relayer/target/x86_64-unknown-linux-gnu/release/relayer /relayer/target/release/

# Make necessary files executable
RUN chmod +x /relayer/src/circom_proofgen.sh
