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

fn get_station_info(code: &str) -> (&'static str, &'static str, &'static str, f64, &'static str) {
    match code {
        "004" => ("1225531", "สถานีสะพานมิตรภาพแม่น้ำสายแห่งที่ 1", "แม่น้ำสาย", 397.59, "maesai_graph.png"),
        "001" => ("1225528", "โจตาดา", "โจตาดา", 514.40, "jotada_graph.png"),
        _ => ("", "", "", 0.0, ""),
    }
}

#[tokio::main]
async fn main() {
    std::fs::create_dir_all("images").ok();

    let app = Router::new()
        .route("/", get(serve_ui))
        .route("/api/text/:code", get(run_text))
        .route("/api/image/:code", get(run_image))
        .nest_service("/images", ServeDir::new("images"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("🚀 ระบบพร้อมใช้งาน (อัปเกรดเปรียบเทียบเวลาอิสระ)! ไปที่: http://127.0.0.1:3000");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn serve_ui() -> Html<&'static str> {
    Html(include_str!("../index.html"))
}

// 📝 API ดึงข้อมูลทั้งหมดของวันนี้ส่งไปให้ Frontend
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

// 📸 ฟังก์ชันคุมเบราว์เซอร์ Headless Chrome (เดิม)
fn capture_graph_with_cdp(search_name: &str, output_file: &str) -> Result<()> {
    let browser = Browser::new(
        LaunchOptions::default_builder()
            .window_size(Some((1920, 1080)))
            .headless(true)
            .sandbox(false)
            .build()
            .unwrap(),
    )?;

    let tab = browser.new_tab()?;
    tab.navigate_to("https://www.thaiwater.net/water/wl")?;
    tab.wait_until_navigated()?;
    std::thread::sleep(StdDuration::from_secs(6));

    let _ = tab.evaluate("document.elementFromPoint(10, 10).click();", false);
    std::thread::sleep(StdDuration::from_secs(1));

    let search_box = tab.wait_for_element("input[placeholder*='ค้นหา']")?;
    search_box.click()?;
    let _ = tab.evaluate("document.querySelector(\"input[placeholder*='ค้นหา']\").value = '';", false);
    search_box.type_into(search_name)?;
    tab.press_key("Enter")?;
    std::thread::sleep(StdDuration::from_secs(2));

    let js_click = format!(r#"
        var btns = document.evaluate("//tr[contains(., '{}')]//button[contains(@title, 'กราฟ')]", document, null, XPathResult.ANY_TYPE, null);
        var btn = btns.iterateNext();
        if (btn) btn.click();
    "#, search_name);
    let _ = tab.evaluate(&js_click, false);

    let _ = tab.wait_for_element(".highcharts-container svg");
    std::thread::sleep(StdDuration::from_secs(4));

    let js_force_tooltip = r#"
        try {
            if (typeof Highcharts !== 'undefined') {
                Highcharts.charts.forEach(function(chart) {
                    if (chart && chart.series && chart.series[0] && chart.series[0].points) {
                        var points = chart.series[0].points;
                        var lastPoint = null;
                        for (var i = points.length - 1; i >= 0; i--) {
                            if (points[i].y !== null && points[i].y !== undefined) {
                                lastPoint = points[i];
                                break;
                            }
                        }
                        if (lastPoint) {
                            chart.tooltip.refresh(lastPoint);
                            lastPoint.setState('hover');
                        }
                    }
                });
            }
        } catch(e) { console.error(e); }

        var svg = document.querySelector('.highcharts-container svg');
        if (svg) {
            var rect = svg.getBoundingClientRect();
            var cx = rect.x + (rect.width * 0.5);
            var ex = rect.x + (rect.width * 0.98);
            var ey = rect.y + (rect.height * 0.45);
            function dispatchMouse(x, y) {
                var el = document.elementFromPoint(x, y) || svg;
                ['pointerover', 'pointerenter', 'mouseenter', 'mouseover', 'mousemove', 'pointermove'].forEach(function(t) {
                    el.dispatchEvent(new MouseEvent(t, { bubbles: true, cancelable: true, clientX: x, clientY: y, view: window }));
                });
            }
            dispatchMouse(cx, ey);
            setTimeout(function() { dispatchMouse(ex, ey); }, 500);
        }
    "#;
    let _ = tab.evaluate(js_force_tooltip, false);
    std::thread::sleep(StdDuration::from_secs(3));

    let js_modal = r#"
        var els = document.querySelectorAll('*');
        var found = false;
        for (var i = 0; i < els.length; i++) {
            var el = els[i];
            if (el.innerText && el.innerText.includes('กราฟระดับน้ำ')) {
                var rect = el.getBoundingClientRect();
                if (rect.width > 600 && rect.height > 400 && rect.width < 1400) {
                    el.id = 'target-modal-for-screenshot';
                    el.scrollIntoView({block: 'center'});
                    found = true;
                    break;
                }
            }
        }
        if (!found) {
            var fallback = document.querySelector('.highcharts-container');
            if (fallback) fallback.id = 'target-modal-for-screenshot';
        }
    "#;
    let _ = tab.evaluate(js_modal, false);
    std::thread::sleep(StdDuration::from_secs(1));

    let png_data = if let Ok(modal) = tab.wait_for_element("#target-modal-for-screenshot") {
        modal.capture_screenshot(CaptureScreenshotFormatOption::Png)?
    } else {
        tab.capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true)?
    };

    std::fs::write(output_file, png_data)?;
    Ok(())
}

// 📝 ดึงข้อมูลของวันนี้ และส่งเป็น Array ให้หน้าเว็บ
async fn fetch_text_summary(station_id: &str, name: &str, bank_level: f64) -> Result<TextResponse, String> {
    let now = Local::now();
    let end_date = now.format("%Y-%m-%d").to_string();
    let start_date = (now - Duration::days(5)).format("%Y-%m-%d").to_string();
    let url = format!("https://api-v3.thaiwater.net/api/v1/thaiwater30/public/waterlevel_graph?station_type=tele_waterlevel&station_id={}&start_date={}&end_date={}", station_id, start_date, end_date);
    
    let client = reqwest::Client::new();
    let res = client.get(&url).header("User-Agent", "Mozilla/5.0").send().await.map_err(|_| "Connection Failed")?;
    
    if let Ok(json) = res.json::<serde_json::Value>().await {
        if let Some(graph_data) = json["data"]["graph_data"].as_array() {
            let valid_data: Vec<_> = graph_data.iter().filter(|d| !d["value"].is_null()).collect();
            
            if valid_data.is_empty() { return Err("ไม่มีข้อมูล".to_string()); }

            let last_datetime = valid_data.last().unwrap()["datetime"].as_str().unwrap_or("");
            let latest_date = if last_datetime.len() >= 10 { &last_datetime[0..10] } else { &end_date };

            let mut results = Vec::new();

            for curr in valid_data.iter() {
                let ct_full = curr["datetime"].as_str().unwrap_or("");
                if !ct_full.starts_with(latest_date) { continue; } // เอาเฉพาะของวันนี้

                let cv = curr["value"].as_f64().unwrap_or(0.0);
                let ct_display = if ct_full.len() >= 16 { &ct_full[11..16] } else { "00:00" };

                results.push(DataPoint {
                    time: ct_display.to_string(),
                    value: cv,
                });
            }

            results.reverse(); // เรียงจากเวลาล่าสุดขึ้นก่อน

            return Ok(TextResponse {
                name: name.to_string(),
                bank_level,
                data_points: results,
            });
        }
    }
    Err("ดึงข้อมูลไม่สำเร็จ".to_string())
}