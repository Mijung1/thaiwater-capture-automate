# 1. Stage สำหรับ Build
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /app
COPY . .

# 💡 แก้ไข: ติดตั้ง pkg-config, libssl-dev (OpenSSL) และ g++ สำหรับการ Build
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    g++ \
    && rm -rf /var/lib/apt/lists/*

# สั่ง Build โดยจำกัดการทำงานเหลือ 1 งานเพื่อไม่ให้ RAM เต็มบน Render แผนฟรี
RUN cargo build --release --jobs 1

# 2. Stage สำหรับรันจริง
FROM debian:bookworm-slim
WORKDIR /app

# ติดตั้ง Chromium, ฟอนต์ไทย และ OpenSSL runtime (libssl3)
RUN apt-get update && apt-get install -y \
    chromium \
    fonts-thai-tlwg \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

ENV CHROME_EXECUTABLE=/usr/bin/chromium

# ตรวจสอบชื่อไฟล์ใน target/release ให้ตรงกับชื่อโปรเจกต์ใน Cargo.toml
# หากใน Cargo.toml ชื่อ "water_monitor_web" ให้ใช้ชื่อนั้น
COPY --from=builder /app/target/release/water_monitor_web /app/
COPY index.html /app/

EXPOSE 3000
CMD ["./water_monitor_web"]