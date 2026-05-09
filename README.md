# 💧 ระบบติดตามระดับน้ำ (Water Level Monitor)

Web Application สำหรับติดตาม แจ้งเตือน และบันทึกภาพกราฟระดับน้ำจากเว็บไซต์ [Thaiwater.net](https://www.thaiwater.net/) แบบอัตโนมัติ พัฒนาด้วยภาษา **Rust** เพื่อประสิทธิภาพขั้นสูงสุด

![Preview](images/maesai_graph.png) ## ✨ ฟีเจอร์หลัก (Features)
- 📝 **ดึงข้อมูลข้อความ (Text Summary):** ดึงข้อมูลระดับน้ำปัจจุบัน เทียบกับเวลาที่ผู้ใช้เลือกในวันเดียวกัน พร้อมคำนวณสถานะ (เพิ่มขึ้น/ลดลง/ทรงตัว) และเปรียบเทียบกับระดับตลิ่ง
- 🖼 **บันทึกภาพกราฟ (Auto Screenshot):** ใช้ Headless Chrome รันเบราว์เซอร์ซ่อนตัวเบื้องหลัง เพื่อดึงภาพกราฟ Highcharts จากเว็บต้นฉบับเป๊ะๆ 100% พร้อมบังคับแสดง Tooltip ตัวเลข
- ⚡ **ความเร็วสูง & ใช้ทรัพยากรน้อย:** เขียนด้วย Rust (Axum + Tokio) และไม่ใช้รันไทม์ที่หนักหน่วง
- ☁️ **Cloud-Ready:** รองรับการ Deploy บน Docker Environment (เช่น Render.com) ทันที

## 🛠️ เทคโนโลยีที่ใช้ (Tech Stack)
- **Backend:** Rust, Axum, Tokio
- **Web Scraping/Automation:** `headless_chrome` (ผ่าน CDP - Chrome DevTools Protocol)
- **Frontend:** HTML, TailwindCSS (ผ่าน CDN), JavaScript (Vanilla)
- **Deployment:** Docker, Debian Bookworm (พร้อมฟอนต์ภาษาไทย `fonts-thai-tlwg`)

## 📍 สถานีที่รองรับปัจจุบัน
1. `004` - สถานีสะพานมิตรภาพแม่น้ำสายแห่งที่ 1 (จ.เชียงราย)
2. `001` - โจตาดา / บ้านแม่สามแลบ (จ.แม่ฮ่องสอน)

## 🚀 วิธีการรันบนเครื่องตัวเอง (Local Development)

### สิ่งที่ต้องมี
- [Rust & Cargo](https://www.rust-lang.org/tools/install)
- Google Chrome ติดตั้งอยู่ในเครื่อง

### ขั้นตอนการรัน
1. Clone repository นี้
   ```bash
   git clone [https://github.com/ช](https://github.com/ช)ื่อผู้ใช้ของคุณ/water_monitor_web.git
   cd water_monitor_web