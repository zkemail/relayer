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

# Clone zk email
RUN git clone https://github.com/zkemail/zk-email-verify -b refactor /zk-email-verify
COPY ./zk-email-verify/build /zk-email-verify/build
WORKDIR /zk-email-verify
RUN yarn install

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

# Clone the repository and set it as the working directory (uses localdir due to no-oss)
# RUN git clone https://github.com/zkemail/relayer /relayer
COPY ./relayer/target /relayer/target
COPY ./relayer/src /relayer/src
COPY ./relayer/abi /relayer/abi
COPY ./relayer/received_eml/.placeholder /relayer/received_eml/.placeholder
WORKDIR /relayer
RUN cargo install
RUN cargo build --release
RUN cargo build

# Make necessary files executable
RUN chmod +x /relayer/src/circom_proofgen.sh
