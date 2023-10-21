FROM aayushg0/rapidsnark:latest AS rapidsnark
FROM aayushg0/relayer:v0 as relayer
FROM aayushg0/zk-email-wallet:v0 as zk-email-wallet
FROM rust:latest
ARG ZKEMAIL_BRANCH_NAME=anon_wallet
ARG CIRCUIT_NAME=wallet
# ARG RELAYER_BRANCH_NAME=modal_anon
ARG ZKEMAIL_COMMIT=e6592d86cb200d98d46db62d63404e7214a11569

RUN apt-get update && apt-get upgrade -y 

# Update the package list and install necessary dependencies
RUN apt-get update && \
  apt install -y nodejs npm cmake build-essential pkg-config libssl-dev libgmp-dev libsodium-dev nasm git awscli

RUN npm install -g yarn npx
 
# Clone rapidsnark repository at the latest commit and set it as the working directory
COPY --from=rapidsnark /rapidsnark/build /rapidsnark/build
WORKDIR /rapidsnark/build
RUN chmod +x /rapidsnark/build/prover

# Clone zk email repository at the latest commit and set it as the working directory
COPY --from=zk-email-wallet /zk-email-verify /zk-email-verify
WORKDIR /zk-email-verify
RUN yarn install
RUN yarn add tsx psl

# Clone the relayer repository at the latest commit and set it as the working directory
COPY --from=relayer /relayer /relayer
RUN chmod +x /relayer/target/release/relayer

# Make necessary files executable
RUN chmod +x /relayer/src/circom_proofgen.sh