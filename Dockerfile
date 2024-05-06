# Build stage with Debian
FROM rust:1.75.0-buster as builder
RUN apt-get update && apt-get upgrade -y
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM rust:1.75.0-buster as facade-service
# WORKDIR /usr/local/bin/
COPY --from=builder /usr/src/app/target/release/facade-service /usr/local/bin/facade-service
# Define the binary as the entrypoint
CMD ["facade-service"]

FROM rust:1.75.0-buster as logging-service
# WORKDIR /usr/local/bin/
COPY --from=builder /usr/src/app/target/release/logging-service /usr/local/bin/logging-service
# Define the binary as the entrypoint
CMD ["logging-service"]

FROM rust:1.75.0-buster as messages-service
# WORKDIR /usr/local/bin/
COPY --from=builder /usr/src/app/target/release/messages-service /usr/local/bin/messages-service
# Define the binary as the entrypoint
CMD ["messages-service"]
