# 1. Stage สำหรับ Build
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /app
COPY . .

# ติดตั้งเครื่องมือช่วยคอมไพล์ HTTPS และ OpenSSL
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    g++ \
    && rm -rf /var/lib/apt/lists/*

# สั่ง Build โดยจำกัดการทำงานเหลือ 1 งาน
RUN cargo build --release --jobs 1

# 2. Stage สำหรับรันจริง
FROM debian:bookworm-slim
WORKDIR /app

# ติดตั้ง Chromium และฟอนต์ไทย
RUN apt-get update && apt-get install -y \
    chromium \
    fonts-thai-tlwg \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

ENV CHROME_EXECUTABLE=/usr/bin/chromium

# ก๊อปปี้ไฟล์ที่ Build เสร็จแล้วมา (ชื่อต้องตรงกับ name ใน Cargo.toml)
COPY --from=builder /app/target/release/water_monitor_web /app/
COPY index.html /app/

EXPOSE 3000
CMD ["./water_monitor_web"]