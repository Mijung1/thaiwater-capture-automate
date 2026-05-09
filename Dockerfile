# 1. Stage สำหรับ Build
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /app
COPY . .

# ติดตั้งเครื่องมือพื้นฐาน
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    g++ \
    && rm -rf /var/lib/apt/lists/*

# 💡 ท่าไม้ตายประหยัด RAM:
# - codegen-units=1: ช่วยลดการใช้ RAM ตอนคอมไพล์ (แต่ใช้เวลานานขึ้น)
# - panic='abort': ลดขนาด Binary และลดการใช้ทรัพยากร
ENV CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
ENV CARGO_PROFILE_RELEASE_PANIC=abort

RUN cargo build --release --jobs 1

# 2. Stage สำหรับรันจริง
FROM debian:bookworm-slim
WORKDIR /app

# ติดตั้ง Chromium, ฟอนต์ไทย และ OpenSSL runtime
RUN apt-get update && apt-get install -y \
    chromium \
    fonts-thai-tlwg \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

ENV CHROME_EXECUTABLE=/usr/bin/chromium

# ตรวจสอบชื่อไฟล์ให้ตรงกับชื่อโปรเจกต์ใน Cargo.toml
COPY --from=builder /app/target/release/water_monitor_web /app/
COPY index.html /app/

EXPOSE 3000
CMD ["./water_monitor_web"]