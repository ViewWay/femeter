//! Web UI — 实时电表监控与控制
//!
//! 基于 axum 的嵌入式 Web 服务器:
//! - / 实时状态页面 (自动刷新)
//! - /control 控制面板
//! - /dlms DLMS 查询
//! - /events 事件日志
//! - /api/status JSON API
//! - /api/dlms?obis=... DLMS 查询 API

use crate::{create_dlms_processor, MeterHandle};
use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub struct WebServer {
    meter: Arc<MeterHandle>,
}

impl WebServer {
    pub fn new(meter: MeterHandle) -> Self {
        Self { meter: Arc::new(meter) }
    }

    pub async fn start(self, port: u16) {
        let stateless = Router::new()
            .route("/", get(serve_index))
            .route("/control", get(serve_control))
            .route("/dlms", get(serve_dlms))
            .route("/events", get(serve_events));
        let api = Router::new()
            .route("/api/status", get(api_status))
            .route("/api/dlms", get(api_dlms))
            .route("/api/set", post(api_set))
            .route("/api/events", get(api_events))
            .with_state(self.meter.clone())
            .layer(CorsLayer::permissive());
        let app = stateless.merge(api);

        let addr = format!("0.0.0.0:{}", port);
        tracing::info!("[Web] Starting on http://{}", addr);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}

// ─── Static page handlers (stateless) ───

async fn serve_index() -> Html<&'static str> { Html(INDEX_HTML) }
async fn serve_control() -> Html<&'static str> { Html(CONTROL_HTML) }
async fn serve_dlms() -> Html<&'static str> { Html(DLMS_HTML) }
async fn serve_events() -> Html<&'static str> { Html(EVENTS_HTML) }

// ─── API handlers ───

async fn api_status(
    axum::extract::State(meter): axum::extract::State<Arc<MeterHandle>>,
) -> impl IntoResponse {
    let mut meter = meter.lock().expect("mutex poisoned");
    let snap = meter.snapshot();
    let cfg = meter.config();
    Json(serde_json::json!({
        "timestamp": snap.timestamp.to_rfc3339(),
        "chip": format!("{:?}", snap.chip),
        "freq": snap.freq,
        "accel": cfg.time_accel,
        "voltage": {
            "a": snap.phase_a.voltage,
            "b": snap.phase_b.voltage,
            "c": snap.phase_c.voltage,
        },
        "current": {
            "a": snap.phase_a.current,
            "b": snap.phase_b.current,
            "c": snap.phase_c.current,
        },
        "angle": {
            "a": snap.phase_a.angle,
            "b": snap.phase_b.angle,
            "c": snap.phase_c.angle,
        },
        "power": {
            "active": snap.computed.p_total,
            "reactive": snap.computed.q_total,
            "apparent": snap.computed.s_total,
            "pf": snap.computed.pf_total,
        },
        "energy": {
            "wh_total": snap.energy.wh_total,
            "varh_total": snap.energy.varh_total,
        },
    }))
}

#[derive(Deserialize)]
struct DlmsQuery {
    obis: String,
}

async fn api_dlms(
    axum::extract::State(meter): axum::extract::State<Arc<MeterHandle>>,
    Query(q): Query<DlmsQuery>,
) -> (StatusCode, Json<Value>) {
    let proc = create_dlms_processor((*meter).clone());
    match proc.query_obis(&q.obis) {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!({"obis": q.obis, "result": result}))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))),
    }
}

#[derive(Deserialize)]
struct SetParams {
    param: String,
    value: String,
}

async fn api_set(
    axum::extract::State(meter): axum::extract::State<Arc<MeterHandle>>,
    axum::extract::Json(params): axum::extract::Json<SetParams>,
) -> (StatusCode, Json<Value>) {
    let mut meter = meter.lock().expect("mutex poisoned");
    let msg = match params.param.as_str() {
        "ua" => { let v: f64 = params.value.parse().unwrap_or(220.0); meter.set_voltage('a', v); format!("A-phase voltage: {:.2}V", v) }
        "ub" => { let v: f64 = params.value.parse().unwrap_or(220.0); meter.set_voltage('b', v); format!("B-phase voltage: {:.2}V", v) }
        "uc" => { let v: f64 = params.value.parse().unwrap_or(220.0); meter.set_voltage('c', v); format!("C-phase voltage: {:.2}V", v) }
        "ia" => { let v: f64 = params.value.parse().unwrap_or(0.0); meter.set_current('a', v); format!("A-phase current: {:.3}A", v) }
        "ib" => { let v: f64 = params.value.parse().unwrap_or(0.0); meter.set_current('b', v); format!("B-phase current: {:.3}A", v) }
        "ic" => { let v: f64 = params.value.parse().unwrap_or(0.0); meter.set_current('c', v); format!("C-phase current: {:.3}A", v) }
        "freq" => { let v: f64 = params.value.parse().unwrap_or(50.0); meter.set_freq(v); format!("Frequency: {:.2}Hz", v) }
        "pf" => { let pf: f64 = params.value.parse().unwrap_or(0.95); let angle = pf.acos() * 180.0 / std::f64::consts::PI; meter.set_angle('a', angle); meter.set_angle('b', angle); meter.set_angle('c', angle); format!("PF: {:.3}", pf) }
        "accel" => { let v: f64 = params.value.parse().unwrap_or(1.0); meter.set_time_accel(v); format!("Accel: {:.0}x", v) }
        "noise" => { let e = ["on","1","true"].contains(&params.value.to_lowercase().as_str()); meter.set_noise(e); format!("Noise: {}", if e {"on"} else {"off"}) }
        "scenario" => { let sc = match params.value.to_lowercase().as_str() { "normal" => crate::Scenario::Normal, "full" | "fullload" => crate::Scenario::FullLoad, "noload" => crate::Scenario::NoLoad, "overv" => crate::Scenario::OverVoltage, "underv" => crate::Scenario::UnderVoltage, "loss" => crate::Scenario::PhaseLoss, "overi" => crate::Scenario::OverCurrent, "reverse" => crate::Scenario::ReversePower, "unbalanced" => crate::Scenario::Unbalanced, _ => { return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "unknown scenario"}))); } }; meter.load_scenario(sc); format!("Scenario: {:?}", sc) }
        _ => { return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("unknown param: {}", params.param)}))); }
    };
    (StatusCode::OK, Json(serde_json::json!({"ok": true, "result": msg})))
}

async fn api_events(
    axum::extract::State(meter): axum::extract::State<Arc<MeterHandle>>,
) -> Json<Value> {
    let meter = meter.lock().expect("mutex poisoned");
    let events = meter.events();
    let arr: Vec<Value> = events.iter().rev().take(50).map(|e| {
        serde_json::json!({
            "timestamp": e.timestamp.to_rfc3339(),
            "event": format!("{:?}", e.event),
            "description": e.description,
        })
    }).collect();
    Json(serde_json::json!({"events": arr}))
}

// ─── Static HTML ───

static INDEX_HTML: &str = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>FeMeter - Real-time Monitor</title>
<meta http-equiv="refresh" content="1">
<style>
*{margin:0;padding:0;box-sizing:border-box}body{font-family:-apple-system,sans-serif;background:#0a0e17;color:#e0e0e0;padding:20px}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:16px;margin-top:16px}
.card{background:#141b2d;border:1px solid #1e2a42;border-radius:8px;padding:16px}
.card h3{color:#4fc3f7;font-size:13px;text-transform:uppercase;letter-spacing:1px;margin-bottom:12px}
.row{display:flex;justify-content:space-between;padding:4px 0;border-bottom:1px solid #1a2238}
.row .label{color:#8892a4;font-size:13px}.row .val{font-size:14px;font-weight:600}
.v-good{color:#66bb6a}.v-warn{color:#ffa726}.v-bad{color:#ef5350}
nav{display:flex;gap:16px;padding:12px 0;border-bottom:1px solid #1e2a42;margin-bottom:8px}
nav a{color:#8892a4;text-decoration:none;font-size:13px}nav a:hover,nav a.active{color:#4fc3f7}
h1{font-size:20px;color:#4fc3f7}
.power-bar{height:6px;background:#1e2a42;border-radius:3px;margin-top:8px;overflow:hidden}
.power-bar-fill{height:100%;border-radius:3px;transition:width 0.5s}
</style></head><body>
<h1>⚡ FeMeter Virtual Meter</h1>
<nav><a href="/" class="active">Status</a><a href="/control">Control</a><a href="/dlms">DLMS</a><a href="/events">Events</a></nav>
<div class="grid" id="data"></div>
<script>
fetch('/api/status').then(r=>r.json()).then(d=>{
  const pkw=(d.power.active/1000).toFixed(2);
  const mkw=60;
  const pfColor=d.power.pf>0.9?'v-good':d.power.pf>0.7?'v-warn':'v-bad';
  const pfClass=d.power.pf>=0?'v-good':'v-bad';
  document.getElementById('data').innerHTML=`
  <div class="card"><h3>⚡ Power</h3>
    <div class="row"><span class="label">Active</span><span class="val">${d.power.active.toFixed(1)} W</span></div>
    <div class="row"><span class="label">Reactive</span><span class="val">${d.power.reactive.toFixed(1)} var</span></div>
    <div class="row"><span class="label">Apparent</span><span class="val">${d.power.apparent.toFixed(1)} VA</span></div>
    <div class="row"><span class="label">Power Factor</span><span class="val ${pfClass}">${d.power.pf.toFixed(4)}</span></div>
    <div class="power-bar"><div class="power-bar-fill" style="width:${Math.min(100,Math.abs(pkw)/mkw*100)}%;background:${d.power.active>=0?'#66bb6a':'#ef5350'}"></div></div>
  </div>
  <div class="card"><h3>🔌 Voltage (V)</h3>
    <div class="row"><span class="label">Phase A</span><span class="val">${d.voltage.a.toFixed(2)}</span></div>
    <div class="row"><span class="label">Phase B</span><span class="val">${d.voltage.b.toFixed(2)}</span></div>
    <div class="row"><span class="label">Phase C</span><span class="val">${d.voltage.c.toFixed(2)}</span></div>
    <div class="row"><span class="label">Frequency</span><span class="val">${d.freq.toFixed(2)} Hz</span></div>
  </div>
  <div class="card"><h3>📊 Current (A)</h3>
    <div class="row"><span class="label">Phase A</span><span class="val">${d.current.a.toFixed(3)}</span></div>
    <div class="row"><span class="label">Phase B</span><span class="val">${d.current.b.toFixed(3)}</span></div>
    <div class="row"><span class="label">Phase C</span><span class="val">${d.current.c.toFixed(3)}</span></div>
    <div class="row"><span class="label">Angle A/B/C</span><span class="val">${d.angle.a.toFixed(1)}° / ${d.angle.b.toFixed(1)}° / ${d.angle.c.toFixed(1)}°</span></div>
  </div>
  <div class="card"><h3>🔋 Energy</h3>
    <div class="row"><span class="label">Active (Wh)</span><span class="val">${d.energy.wh_total.toFixed(2)}</span></div>
    <div class="row"><span class="label">Reactive (varh)</span><span class="val">${d.energy.varh_total.toFixed(2)}</span></div>
    <div class="row"><span class="label">Active (kWh)</span><span class="val">${(d.energy.wh_total/1000).toFixed(4)}</span></div>
    <div class="row"><span class="label">Chip</span><span class="val">${d.chip}</span></div>
    <div class="row"><span class="label">Accel</span><span class="val">${d.accel}x</span></div>
  </div>`;
});
</script></body></html>"#;

static CONTROL_HTML: &str = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>FeMeter - Control</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}body{font-family:-apple-system,sans-serif;background:#0a0e17;color:#e0e0e0;padding:20px}
nav{display:flex;gap:16px;padding:12px 0;border-bottom:1px solid #1e2a42;margin-bottom:16px}
nav a{color:#8892a4;text-decoration:none;font-size:13px}nav a:hover,nav a.active{color:#4fc3f7}
h1{font-size:20px;color:#4fc3f7;margin-bottom:16px}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:16px}
.card{background:#141b2d;border:1px solid #1e2a42;border-radius:8px;padding:16px}
.card h3{color:#4fc3f7;font-size:13px;text-transform:uppercase;letter-spacing:1px;margin-bottom:12px}
label{display:block;color:#8892a4;font-size:12px;margin:8px 0 4px}
input,select{background:#1a2238;border:1px solid #2a3a5c;color:#e0e0e0;padding:8px 12px;border-radius:4px;width:100%;font-size:14px}
input:focus,select:focus{outline:none;border-color:#4fc3f7}
.btn{background:#1565c0;color:white;border:none;padding:10px 20px;border-radius:4px;cursor:pointer;font-size:14px;margin-top:12px;width:100%}
.btn:hover{background:#1976d2}.btn-sm{padding:6px 12px;font-size:12px;width:auto;margin:4px}
.scenarios{display:flex;flex-wrap:wrap;gap:8px;margin-top:8px}
#result{margin-top:16px;padding:12px;background:#1a2238;border-radius:4px;font-size:13px;min-height:40px;color:#66bb6a}
</style></head><body>
<h1>⚡ FeMeter Control</h1>
<nav><a href="/">Status</a><a href="/control" class="active">Control</a><a href="/dlms">DLMS</a><a href="/events">Events</a></nav>
<div class="grid">
  <div class="card"><h3>Voltage / Current</h3>
    <label>Voltage A (V)</label><input id="ua" type="number" step="0.1" value="220">
    <label>Voltage B (V)</label><input id="ub" type="number" step="0.1" value="220">
    <label>Voltage C (V)</label><input id="uc" type="number" step="0.1" value="220">
    <label>Current A (A)</label><input id="ia" type="number" step="0.01" value="5">
    <label>Current B (A)</label><input id="ib" type="number" step="0.01" value="5">
    <label>Current C (A)</label><input id="ic" type="number" step="0.01" value="5">
    <button class="btn" onclick="setMany(['ua','ub','uc','ia','ib','ic'])">Apply All</button>
  </div>
  <div class="card"><h3>Frequency / PF / Accel</h3>
    <label>Frequency (Hz)</label><input id="freq" type="number" step="0.1" value="50">
    <label>Power Factor</label><input id="pf" type="number" step="0.01" min="-1" max="1" value="0.95">
    <label>Time Acceleration</label><input id="accel" type="number" step="1" value="1">
    <button class="btn" onclick="setMany(['freq','pf','accel'])">Apply</button>
  </div>
  <div class="card"><h3>Scenarios</h3>
    <div class="scenarios">
      <button class="btn btn-sm" onclick="set('scenario','normal')">Normal</button>
      <button class="btn btn-sm" onclick="set('scenario','full')">Full Load</button>
      <button class="btn btn-sm" onclick="set('scenario','noload')">No Load</button>
      <button class="btn btn-sm" onclick="set('scenario','overv')">Over Voltage</button>
      <button class="btn btn-sm" onclick="set('scenario','underv')">Under Voltage</button>
      <button class="btn btn-sm" onclick="set('scenario','loss')">Phase Loss</button>
      <button class="btn btn-sm" onclick="set('scenario','overi')">Over Current</button>
      <button class="btn btn-sm" onclick="set('scenario','reverse')">Reverse Power</button>
      <button class="btn btn-sm" onclick="set('scenario','unbalanced')">Unbalanced</button>
    </div>
  </div>
</div>
<div id="result"></div>
<script>
function set(p,v){fetch('/api/set',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({param:p,value:v})}).then(r=>r.json()).then(d=>{document.getElementById('result').textContent=d.result||d.error})}
function setMany(ids){ids.forEach(id=>{const el=document.getElementById(id);if(el)set(id,el.value)})}
</script></body></html>"#;

static DLMS_HTML: &str = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>FeMeter - DLMS Query</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}body{font-family:-apple-system,sans-serif;background:#0a0e17;color:#e0e0e0;padding:20px}
nav{display:flex;gap:16px;padding:12px 0;border-bottom:1px solid #1e2a42;margin-bottom:16px}
nav a{color:#8892a4;text-decoration:none;font-size:13px}nav a:hover,nav a.active{color:#4fc3f7}
h1{font-size:20px;color:#4fc3f7;margin-bottom:16px}
.card{background:#141b2d;border:1px solid #1e2a42;border-radius:8px;padding:16px;margin-bottom:16px}
.card h3{color:#4fc3f7;font-size:13px;text-transform:uppercase;letter-spacing:1px;margin-bottom:12px}
input{background:#1a2238;border:1px solid #2a3a5c;color:#e0e0e0;padding:8px 12px;border-radius:4px;font-size:14px;width:300px}
input:focus{outline:none;border-color:#4fc3f7}
.btn{background:#1565c0;color:white;border:none;padding:8px 16px;border-radius:4px;cursor:pointer;font-size:14px;margin-left:8px}
.btn:hover{background:#1976d2}
.shortcuts{display:flex;flex-wrap:wrap;gap:6px;margin-top:12px}
.btn-sm{padding:4px 10px;font-size:11px;background:#1a2238;color:#4fc3f7;border:1px solid #2a3a5c;border-radius:3px;cursor:pointer}
.btn-sm:hover{background:#2a3a5c}
#result{margin-top:16px;padding:12px;background:#1a2238;border-radius:4px;font-size:13px;font-family:monospace;white-space:pre-wrap;min-height:60px;color:#a5d6a7}
</style></head><body>
<h1>⚡ DLMS Query</h1>
<nav><a href="/">Status</a><a href="/control">Control</a><a href="/dlms" class="active">DLMS</a><a href="/events">Events</a></nav>
<div class="card"><h3>Query OBIS</h3>
  <input id="obis" placeholder="1.0.81.7.27.255" value="1.0.81.7.27.255">
  <button class="btn" onclick="query()">Query</button>
  <div class="shortcuts">
    <button class="btn-sm" onclick="quick('0.0.1')">0.0.1 Device</button>
    <button class="btn-sm" onclick="quick('1.0.31.7.0.255')">1.0.31.7.0.255 Voltage</button>
    <button class="btn-sm" onclick="quick('1.0.51.7.0.255')">1.0.51.7.0.255 Current</button>
    <button class="btn-sm" onclick="quick('1.0.12.7.0.255')">1.0.12.7.0.255 Freq</button>
    <button class="btn-sm" onclick="quick('1.0.81.7.27.255')">1.0.81.7.27.255 P_total</button>
    <button class="btn-sm" onclick="quick('1.0.82.7.27.255')">1.0.82.7.27.255 Q_total</button>
    <button class="btn-sm" onclick="quick('1.0.83.7.27.255')">1.0.83.7.27.255 S_total</button>
    <button class="btn-sm" onclick="quick('1.1.1.8.0.255')">1.1.1.8.0.255 Wh+</button>
    <button class="btn-sm" onclick="quick('1.1.5.8.0.255')">1.1.5.8.0.255 varh+</button>
    <button class="btn-sm" onclick="quick('0.9.1.0.255.255')">0.9.1.0.255.255 Clock</button>
    <button class="btn-sm" onclick="quick('1.0.13.7.0.255')">1.0.13.7.0.255 PF</button>
    <button class="btn-sm" onclick="quick('1.0.14.7.0.255')">1.0.14.7.0.255 Angle</button>
    <button class="btn-sm" onclick="quick('0.0.96.1.0.255')">0.0.96.1.0.255 Status</button>
  </div>
</div>
<div id="result">Click query or a shortcut...</div>
<script>
function quick(obis){document.getElementById('obis').value=obis;query()}
function query(){const obis=document.getElementById('obis').value;fetch('/api/dlms?obis='+encodeURIComponent(obis)).then(r=>r.json()).then(d=>{document.getElementById('result').textContent=JSON.stringify(d,null,2)}).catch(e=>{document.getElementById('result').textContent='Error: '+e})}
</script></body></html>"#;

static EVENTS_HTML: &str = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>FeMeter - Events</title>
<meta http-equiv="refresh" content="5">
<style>
*{margin:0;padding:0;box-sizing:border-box}body{font-family:-apple-system,sans-serif;background:#0a0e17;color:#e0e0e0;padding:20px}
nav{display:flex;gap:16px;padding:12px 0;border-bottom:1px solid #1e2a42;margin-bottom:16px}
nav a{color:#8892a4;text-decoration:none;font-size:13px}nav a:hover,nav a.active{color:#4fc3f7}
h1{font-size:20px;color:#4fc3f7;margin-bottom:16px}
table{width:100%;border-collapse:collapse;background:#141b2d;border-radius:8px;overflow:hidden}
th{background:#1a2238;color:#4fc3f7;font-size:12px;text-transform:uppercase;letter-spacing:1px;padding:10px 16px;text-align:left}
td{padding:8px 16px;border-bottom:1px solid #1e2a42;font-size:13px}
tr:hover{background:#1a2238}
</style></head><body>
<h1>⚡ Event Log</h1>
<nav><a href="/">Status</a><a href="/control">Control</a><a href="/dlms">DLMS</a><a href="/events" class="active">Events</a></nav>
<table><thead><tr><th>Time</th><th>Event</th><th>Description</th></tr></thead><tbody id="events"></tbody></table>
<script>
fetch('/api/events').then(r=>r.json()).then(d=>{
  document.getElementById('events').innerHTML=d.events.map(e=>`<tr><td>${e.timestamp}</td><td>${e.event}</td><td>${e.description}</td></tr>`).join('')||'<tr><td colspan="3" style="text-align:center;color:#8892a4">No events</td></tr>';
});
</script></body></html>"#;
