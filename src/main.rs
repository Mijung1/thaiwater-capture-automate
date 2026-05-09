use anyhow::Result;
use axum::{
    extract::Path,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use chrono::{Duration, Local};
use headless_chrome::{protocol::cdp::Page::CaptureScreenshotFormatOption, Browser, LaunchOptions};
use serde::Serialize;
use serde_json::json;
use std::net::SocketAddr;
use std::time::Duration as StdDuration;
use tower_http::services::ServeDir;

#[derive(Serialize, Clone)]
struct DataPoint {
    time: String,
    value: f64,
}

#[derive(Serialize)]
struct TextResponse {
    name: String,
    bank_level: f64,
    data_points: Vec<DataPoint>,
}

#[derive(Serialize)]
struct ImageResponse {
    image_url: String,
}

// ข้อมูลสถานีที่รองรับ
fn get_station_info(code: &str) -> (&'static str, &'static str, &'static str, f64, &'static str) {
    match code {
        "004" => ("1225531", "สถานีสะพานมิตรภาพแม่น้ำสายแห่งที่ 1", "แม่น้ำสาย", 397.59, "maesai_graph.png"),
        "001" => ("1225528", "โจตาดา", "โจตาดา", 514.40, "jotada_graph.png"),
        _ => ("", "", "", 0.0, ""),
    }
}

#[tokio::main]
async fn main() {
    // สร้างโฟลเดอร์สำหรับเก็บภาพแคปหน้าจอ
    std::fs::create_dir_all("images").ok();

    let app = Router::new()
        .route("/", get(serve_ui))
        .route("/api/text/:code", get(run_text))
        .route("/api/image/:code", get(run_image))
        .nest_service("/images", ServeDir::new("images"));

    // --- ส่วนสำคัญสำหรับการ Deploy บน Render ---
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    // ต้องใช้ 0.0.0.0 เพื่อให้เข้าถึงจากภายนอกได้
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🚀 Server is starting on {}", addr);
    // ---------------------------------------

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn serve_ui() -> Html<&'static str> {
    // อ่านไฟล์ index.html ที่อยู่โฟลเดอร์เดียวกับโปรเจกต์
    Html(include_str!("../index.html"))
}

// API สำหรับดึงข้อมูลสรุปข้อความ
async fn run_text(Path(code): Path<String>) -> impl IntoResponse {
    let (station_id, _, short_name, bank_level, _) = get_station_info(&code);
    if station_id.is_empty() {
        return Json(json!({"error": "Unknown Code"})).into_response();
    }

    match fetch_text_summary(station_id, short_name, bank_level).await {
        Ok(resp) => Json(json!(resp)).into_response(),
        Err(e) => Json(json!({"error": e})).into_response(),
    }
}

// API สำหรับสั่งแคปภาพกราฟ
async fn run_image(Path(code): Path<String>) -> impl IntoResponse {
    let (_, search_name, short_name, _, img_name) = get_station_info(&code);
    if search_name.is_empty() {
        return Json(json!({"error": "Unknown Code"})).into_response();
    }

    let img_path = format!("images/{}", img_name);
    let search_name_clone = search_name.to_string();
    let img_path_clone = img_path.clone();

    let capture_result = tokio::task::spawn_blocking(move || {
        capture_graph_with_cdp(&search_name_clone, &img_path_clone)
    })
    .await
    .unwrap();

    if let Err(e) = capture_result {
        println!("❌ Error Headless Chrome ({}): {:?}", short_name, e);
        return Json(json!({"error": "Failed to capture image"})).into_response();
    }

    Json(json!(ImageResponse {
        image_url: format!("/{}?t={}", img_path, Local::now().timestamp()),
    })).into_response()
}

// ฟังก์ชันควบคุม Browser สำหรับแคปหน้าจอ
fn capture_graph_with_cdp(search_name: &str, output_file: &str) -> Result<()> {
    let browser = Browser::new(
        LaunchOptions::default_builder()
            .window_size(Some((1280, 800)))
            .headless(true)
            .sandbox(false) // จำเป็นสำหรับการรันบน Docker/Cloud
            .build()
            .unwrap(),
    )?;

    let tab = browser.new_tab()?;
    tab.navigate_to("https://www.thaiwater.net/water/wl")?;
    tab.wait_until_navigated()?;
    std::thread::sleep(StdDuration::from_secs(5));

    // คลิกเพื่อปิด Modal แจ้งเตือนถ้ามี
    let _ = tab.evaluate("document.elementFromPoint(10, 10).click();", false);

    // ค้นหาชื่อสถานี
    let search_box = tab.wait_for_element("input[placeholder*='ค้นหา']")?;
    search_box.click()?;
    search_box.type_into(search_name)?;
    tab.press_key("Enter")?;
    std::thread::sleep(StdDuration::from_secs(2));

    // คลิกปุ่มกราฟ
    let js_click = format!(r#"
        var btns = document.evaluate("//tr[contains(., '{}')]//button[contains(@title, 'กราฟ')]", document, null, XPathResult.ANY_TYPE, null);
        var btn = btns.iterateNext();
        if (btn) btn.click();
    "#, search_name);
    let _ = tab.evaluate(&js_click, false);

    tab.wait_for_element(".highcharts-container svg")?;
    std::thread::sleep(StdDuration::from_secs(3));

    // Force ให้ Tooltip แสดงผลเลขระดับน้ำล่าสุด
    let js_force_tooltip = r#"
        try {
            if (typeof Highcharts !== 'undefined') {
                Highcharts.charts.forEach(function(chart) {
                    if (chart && chart.series && chart.series[0] && chart.series[0].points) {
                        var points = chart.series[0].points;
                        var lastPoint = points.filter(p => p.y !== null).pop();
                        if (lastPoint) {
                            chart.tooltip.refresh(lastPoint);
                            lastPoint.setState('hover');
                        }
                    }
                });
            }
        } catch(e) {}
    "#;
    let _ = tab.evaluate(js_force_tooltip, false);
    std::thread::sleep(StdDuration::from_secs(2));

    // เลือกเฉพาะส่วนของ Modal กราฟเพื่อแคปภาพ
    let js_identify = r#"
        var modal = document.querySelector('.modal-content') || document.querySelector('.highcharts-container');
        if(modal) modal.id = 'target-screenshot';
    "#;
    let _ = tab.evaluate(js_identify, false);

    let png_data = if let Ok(el) = tab.wait_for_element("#target-screenshot") {
        el.capture_screenshot(CaptureScreenshotFormatOption::Png)?
    } else {
        tab.capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true)?
    };

    std::fs::write(output_file, png_data)?;
    Ok(())
}

// ฟังก์ชันดึงข้อมูล JSON จาก API ของ Thaiwater
async fn fetch_text_summary(station_id: &str, name: &str, bank_level: f64) -> Result<TextResponse, String> {
    let now = Local::now();
    let end_date = now.format("%Y-%m-%d").to_string();
    let start_date = (now - Duration::days(1)).format("%Y-%m-%d").to_string();
    
    let url = format!(
        "https://api-v3.thaiwater.net/api/v1/thaiwater30/public/waterlevel_graph?station_type=tele_waterlevel&station_id={}&start_date={}&end_date={}",
        station_id, start_date, end_date
    );
    
    let client = reqwest::Client::new();
    let res = client.get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .map_err(|e| format!("Connection Error: {}", e))?;
    
    let json: serde_json::Value = res.json().await.map_err(|e| format!("JSON Error: {}", e))?;
    
    if let Some(graph_data) = json["data"]["graph_data"].as_array() {
        let mut results = Vec::new();
        let today_str = now.format("%Y-%m-%d").to_string();

        for curr in graph_data {
            let dt = curr["datetime"].as_str().unwrap_or("");
            if !dt.starts_with(&today_str) { continue; } // กรองเอาเฉพาะของวันนี้

            if let Some(val) = curr["value"].as_f64() {
                results.push(DataPoint {
                    time: dt[11..16].to_string(), // ตัดเอาเฉพาะ HH:mm
                    value: val,
                });
            }
        }

        results.reverse(); // เอาเวลาล่าสุดขึ้นก่อน

        Ok(TextResponse {
            name: name.to_string(),
            bank_level,
            data_points: results,
        })
    } else {
        Err("ไม่พบข้อมูลกราฟ".to_string())
    }
}