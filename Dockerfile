FROM rust:1.75-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y \
    chromium \
    fonts-thai-tlwg \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
ENV CHROME_EXECUTABLE=/usr/bin/chromium
COPY --from=builder /app/target/release/water_monitor_web /app/
COPY index.html /app/
EXPOSE 3000
CMD ["./water_monitor_web"]