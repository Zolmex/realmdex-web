use app::server_fns::{list_servers, server_sparkline};
use app::types::{Category, InitialServers};
use leptos::prelude::{provide_context, Owner};
use leptos::tachys::view::RenderHtml;
use worker::*;

mod poller;
mod rollup;
mod security;

#[event(scheduled)]
async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    console_error_panic_hook::set_once();
    let cron = event.cron();
    let res = if cron.starts_with("*/1") {
        poller::run(&env).await
    } else if cron.starts_with("30 3") {
        rollup::run(&env).await
    } else {
        Ok(())
    };
    if let Err(e) = res {
        console_log!("scheduled error: {e}");
    }
}

#[event(fetch)]
async fn fetch(mut req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    if let Err(deny) = security::guard_api(&req) {
        return Ok(deny);
    }
    let path = req.path();

    if path == "/api/list_servers" && req.method() == Method::Post {
        #[derive(serde::Deserialize)]
        struct Body {
            category: Category,
        }
        let body: Body = match req.json().await {
            Ok(b) => b,
            Err(e) => return Ok(security::add_cors(Response::error(format!("bad json: {e}"), 400)?)),
        };
        let owner = Owner::new();
        owner.set();
        provide_context(env);
        let result = list_servers(body.category).await;
        return match result {
            Ok(v) => Ok(security::add_cors(Response::from_json(&v)?)),
            Err(e) => Ok(security::add_cors(Response::error(format!("server fn error: {e}"), 500)?)),
        };
    }

    if path == "/api/server_sparkline" && req.method() == Method::Post {
        #[derive(serde::Deserialize)]
        struct Body {
            server_id: i64,
        }
        let body: Body = match req.json().await {
            Ok(b) => b,
            Err(e) => return Ok(security::add_cors(Response::error(format!("bad json: {e}"), 400)?)),
        };
        let owner = Owner::new();
        owner.set();
        provide_context(env);
        let result = server_sparkline(body.server_id).await;
        return match result {
            Ok(v) => Ok(security::add_cors(Response::from_json(&v)?)),
            Err(e) => Ok(security::add_cors(Response::error(format!("server fn error: {e}"), 500)?)),
        };
    }

    // SSR fallback. Fetch both categories from D1 first, inject into Leptos
    // context, and embed an initial-data JSON blob + a small controller script.
    let db = env.d1("DB")?;
    let pserver = app::db::list_servers_in_category(&db, Category::Pserver)
        .await
        .unwrap_or_default();
    let realm_like = app::db::list_servers_in_category(&db, Category::RealmLike)
        .await
        .unwrap_or_default();
    let initial = InitialServers { pserver, realm_like };
    let initial_json = serde_json::to_string(&initial).unwrap_or_default();

    let initial_for_ctx = initial.clone();
    let owner = Owner::new();
    let html = owner.with(|| {
        provide_context(env);
        provide_context(initial_for_ctx);
        app::App().to_html()
    });

    let body = format!(
        "<!DOCTYPE html><html lang=\"en\"><head>\
            <meta charset=\"utf-8\">\
            <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
            <meta name=\"description\" content=\"Track server status, player counts, and uptime for RotMG private servers and Realm-Like games.\">\
            <title>RealmDex - RotMG Private Server Stats & Uptime</title>\
            <link rel=\"icon\" type=\"image/x-icon\" href=\"/favicon.ico\">\
            <link rel=\"stylesheet\" href=\"/styles/index.css\">\
        </head><body>{html}\
        <script id=\"initial-data\" type=\"application/json\">{initial_json}</script>\
        <script>{controller}</script>\
        </body></html>",
        controller = CLIENT_CONTROLLER
    );
    Ok(security::add_cors(Response::from_html(body)?))
}

// Vanilla-JS controller: handles tab switching, sort, and 30s live updates.
// Fetches /api/list_servers, rebuilds card markup minimally for the active
// category, and re-applies sort. This is intentionally small — full hydration
// will come once we have a workers-compatible Leptos server-fn flavor.
const CLIENT_CONTROLLER: &str = r#"
(function(){
  var initialEl = document.getElementById('initial-data');
  var state = { pserver: [], 'realm-like': [] };
  try {
    var parsed = JSON.parse(initialEl.textContent || '{}');
    state.pserver = parsed.pserver || [];
    state['realm-like'] = parsed.realm_like || [];
  } catch(e) { console.error('initial-data parse', e); }

  var activeCat = 'pserver';
  var activeSort = 'players-desc';

  function uptimeColor(p) {
    if (p >= 99) return '#3fb950';
    if (p >= 95) return '#7cc06b';
    if (p >= 80) return '#d1b03d';
    if (p >= 50) return '#d97935';
    return '#b8423a';
  }
  function avgUptime(s) {
    if (!s.uptime_14d || !s.uptime_14d.length) return 0;
    var t = 0;
    for (var i=0;i<s.uptime_14d.length;i++) t += s.uptime_14d[i].uptime_percent;
    return t / s.uptime_14d.length;
  }
  function hashSeed(s) {
    var h = 2166136261;
    for (var i=0;i<s.length;i++) { h ^= s.charCodeAt(i); h = (h * 16777619) >>> 0; }
    return h;
  }
  function shuffle(arr) {
    // deterministic-enough via xorshift seeded by name hashes
    var seed = arr.reduce(function(a,s){ return (a ^ hashSeed(s.name)) >>> 0; }, 1);
    function rnd(){ seed ^= seed << 13; seed ^= seed >>> 17; seed ^= seed << 5; return (seed>>>0) / 4294967296; }
    var a = arr.slice();
    for (var i=a.length-1;i>0;i--) { var j=Math.floor(rnd()*(i+1)); var t=a[i]; a[i]=a[j]; a[j]=t; }
    return a;
  }
  function sortOnline(online) {
    var a = online.slice();
    if (activeSort === 'players-desc') a.sort(function(x,y){ return y.current_players - x.current_players; });
    else if (activeSort === 'players-asc') a.sort(function(x,y){ return x.current_players - y.current_players; });
    else if (activeSort === 'uptime-desc') a.sort(function(x,y){ return avgUptime(y) - avgUptime(x); });
    else if (activeSort === 'random') a = shuffle(a);
    return a;
  }
  function svgSparkline(points) {
    var w = 120, h = 24;
    if (!points || !points.length) return '<svg class="sparkline" width="'+w+'" height="'+h+'" viewBox="0 0 '+w+' '+h+'"><path d="" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>';
    if (points.length === 1) {
      var y = h/2;
      return '<svg class="sparkline" width="'+w+'" height="'+h+'" viewBox="0 0 '+w+' '+h+'"><path d="M 0 '+y.toFixed(1)+' L '+w.toFixed(1)+' '+y.toFixed(1)+'" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>';
    }
    var min = points[0].players, max = points[0].players;
    for (var i=1;i<points.length;i++) { if (points[i].players<min) min=points[i].players; if (points[i].players>max) max=points[i].players; }
    var range = Math.max(max-min, 1);
    var n = points.length;
    var d = '';
    for (var i=0;i<n;i++) {
      var x = (i/(n-1))*w;
      var y = h - ((points[i].players-min)/range)*h;
      d += (i===0?'M ':' L ') + x.toFixed(1) + ' ' + y.toFixed(1);
    }
    return '<svg class="sparkline" width="'+w+'" height="'+h+'" viewBox="0 0 '+w+' '+h+'"><path d="'+d+'" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>';
  }
  function uptimeGrid(days) {
    var html = '<div class="uptime-grid">';
    for (var i=0;i<days.length;i++) {
      var d = days[i];
      html += '<div class="uptime-day" style="background-color: '+uptimeColor(d.uptime_percent)+'" data-uptime="'+d.uptime_percent+'" data-day="'+(i+1)+'"></div>';
    }
    return html + '</div>';
  }
  function esc(s){ return String(s==null?'':s).replace(/[&<>"']/g, function(c){ return ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]; }); }
  function renderCard(s) {
    var status = s.status; // 'online' | 'offline' | 'wip'
    var isWip = status === 'wip';
    var statusText = isWip ? 'WIP' : (status === 'online' ? 'Online' : 'Offline');
    var linkText = (s.link||'').indexOf('discord') >= 0 ? 'Join Discord' : 'Visit Homepage';
    var players = isWip ? '-' : String(s.current_players);
    var peak = isWip ? '-' : String(s.peak_24h);
    var html = '<div class="server-card" data-server-id="'+s.id+'">'
      + '<div class="card-header">'
        + '<img src="'+esc(s.icon_path)+'" alt="'+esc(s.name)+'" class="server-icon" data-discord="'+esc(s.link)+'"/>'
        + '<div class="server-info">'
          + '<h3 class="server-name">'+esc(s.name)+'</h3>'
          + '<a href="'+esc(s.link)+'" class="server-discord" target="_blank" rel="noopener noreferrer">'+linkText+'</a>'
        + '</div>'
        + '<div class="status-container">'
          + '<div class="status-indicator '+status+'" title="'+statusText+'"></div>'
          + '<span class="status-text '+status+'">'+statusText+'</span>'
        + '</div>'
      + '</div>'
      + '<div class="card-stats">'
        + '<div class="stat-row"><span class="stat-label">Players</span><span class="stat-value">'+players+'</span></div>'
        + '<div class="stat-row"><span class="stat-label">24h Peak</span><span class="stat-value">'+peak+'</span></div>'
      + '</div>';
    if (!isWip) {
      var week = (s.uptime_14d||[]).slice(0,7);
      var two = (s.uptime_14d||[]);
      html += '<div class="sparkline-wrapper">'+svgSparkline(s.sparkline||[])+'</div>'
        + '<div class="uptime-section"><div class="uptime-wrapper">'
        + '<div class="uptime-labels">'
        + '<div class="uptime-label uptime-label-week">Uptime (Past Week)</div>'
        + '<div class="uptime-label uptime-label-2week">Uptime (Past 2 Weeks)</div>'
        + '</div>'
        + '<div class="uptime-grids">'
        + '<div class="uptime-week">'+uptimeGrid(week)+'</div>'
        + '<div class="uptime-2week">'+uptimeGrid(two)+'</div>'
        + '</div></div></div>';
    }
    return html + '</div>';
  }
  function renderGrid() {
    var data = state[activeCat] || [];
    var online = [], offline = [], wip = [];
    for (var i=0;i<data.length;i++) {
      var s = data[i];
      if (s.status === 'online') online.push(s);
      else if (s.status === 'offline') offline.push(s);
      else if (s.status === 'wip') wip.push(s);
    }
    online = sortOnline(online);
    var html = '';
    for (var i=0;i<online.length;i++) html += renderCard(online[i]);
    if (offline.length) {
      html += '<div class="wip-divider offline-divider"><span>Offline</span></div>';
      for (var i=0;i<offline.length;i++) html += renderCard(offline[i]);
    }
    if (wip.length) {
      html += '<div class="wip-divider"><span>Work in Progress</span></div>';
      for (var i=0;i<wip.length;i++) html += renderCard(wip[i]);
    }
    var grid = document.querySelector('.server-grid');
    if (grid) {
      grid.setAttribute('data-category', activeCat);
      grid.innerHTML = html;
    }
  }
  function refetch() {
    fetch('/api/list_servers', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({ category: activeCat })
    }).then(function(r){ return r.ok ? r.json() : null; })
      .then(function(data){ if (data) { state[activeCat] = data; renderGrid(); } })
      .catch(function(e){ console.error('refetch', e); });
  }
  document.addEventListener('click', function(e){
    var t = e.target;
    if (t && t.classList && t.classList.contains('category-tab')) {
      var cat = t.getAttribute('data-category');
      if (!cat || cat === activeCat) return;
      activeCat = cat;
      var tabs = document.querySelectorAll('.category-tab');
      for (var i=0;i<tabs.length;i++) tabs[i].classList.toggle('active', tabs[i].getAttribute('data-category') === activeCat);
      renderGrid();
      refetch();
    }
  });
  var sel = document.getElementById('sort-select');
  if (sel) sel.addEventListener('change', function(){ activeSort = sel.value; renderGrid(); });
  setInterval(refetch, 30000);
})();
"#;
