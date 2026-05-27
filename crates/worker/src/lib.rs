use app::server_fns::{list_servers, server_sparkline};
use app::types::{Category, InitialServers};
use leptos::prelude::{provide_context, Owner};
use leptos::tachys::view::RenderHtml;
use worker::*;

mod admin;
mod poller;
mod rollup;
mod security;

pub(crate) fn safe_json<T: serde::Serialize>(val: &T) -> String {
    // "</" -> "<\/" prevents </script> breakout in embedded JSON
    serde_json::to_string(val).unwrap_or_default().replace("</", "<\\/")
}

pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&#39;")
}

pub(crate) fn html_shell(title: &str, head_extra: &str, content: &str, data_id: &str, data_json: &str, controller: &str) -> String {
    let safe_title = html_escape(title);
    let safe_data_id = html_escape(data_id);
    format!(
        "<!DOCTYPE html><html lang=\"en\"><head>\
            <meta charset=\"utf-8\">\
            <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
            {head_extra}\
            <title>{safe_title}</title>\
            <link rel=\"icon\" type=\"image/x-icon\" href=\"/favicon.ico\">\
            <link rel=\"stylesheet\" href=\"/styles/index.css\">\
        </head><body>{content}\
        <script id=\"{safe_data_id}\" type=\"application/json\">{data_json}</script>\
        <script>{controller}</script>\
        </body></html>"
    )
}

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

async fn json_api<B, T, F, Fut>(req: &mut Request, env: Env, f: F) -> Result<Response>
where
    B: serde::de::DeserializeOwned,
    T: serde::Serialize,
    F: FnOnce(B) -> Fut,
    Fut: std::future::Future<Output = worker::Result<T>>,
{
    let body: B = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            console_log!("bad json: {e}");
            return Ok(security::add_cors(Response::error("invalid request", 400)?));
        }
    };
    let owner = Owner::new();
    owner.set();
    provide_context(env);
    match f(body).await {
        Ok(v) => Ok(security::add_cors(Response::from_json(&v)?)),
        Err(e) => {
            console_log!("server fn error: {e}");
            Ok(security::add_cors(Response::error("internal error", 500)?))
        }
    }
}

#[event(fetch)]
async fn fetch(mut req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    if let Err(deny) = security::guard_api(&req) {
        return Ok(deny);
    }
    let path = req.path();

    if path.starts_with("/admin") || path.starts_with("/api/admin/") {
        let email = match security::guard_admin(&req, &env).await {
            Ok(e) => e,
            Err(deny) => return Ok(deny),
        };
        return admin::handle(&mut req, &env, &path, &email).await;
    }

    if path == "/api/list_servers" && req.method() == Method::Post {
        #[derive(serde::Deserialize)]
        struct Body { category: Category }
        return json_api(&mut req, env, |b: Body| list_servers(b.category)).await;
    }

    if path == "/api/server_sparkline" && req.method() == Method::Post {
        #[derive(serde::Deserialize)]
        struct Body { server_id: i64 }
        return json_api(&mut req, env, |b: Body| server_sparkline(b.server_id)).await;
    }

    let db = env.d1("DB")?;
    let pserver = app::db::list_servers_in_category(&db, Category::Pserver)
        .await
        .unwrap_or_default();
    let realm_like = app::db::list_servers_in_category(&db, Category::RealmLike)
        .await
        .unwrap_or_default();
    let initial = InitialServers { pserver, realm_like };
    let initial_json = safe_json(&initial);

    let owner = Owner::new();
    let content = owner.with(|| app::App().to_html());

    let body = html_shell(
        "RealmDex - RotMG Private Server Stats & Uptime",
        "<meta name=\"description\" content=\"Track server status, player counts, and uptime for RotMG private servers and Realm-Like games.\">",
        &content,
        "initial-data",
        &initial_json,
        CLIENT_CONTROLLER,
    );
    Ok(security::add_cors(Response::from_html(body)?))
}

const ADMIN_CONTROLLER: &str = r#"
(function(){
  var dataEl = document.getElementById('admin-data');
  if (!dataEl) return;
  var data;
  try { data = JSON.parse(dataEl.textContent || '{}'); } catch(e) { return; }

  function esc(s){ return String(s==null?'':s).replace(/[&<>"']/g, function(c){ return ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]; }); }

  function flash(msg, ok) {
    var el = document.getElementById('admin-flash');
    if (!el) return;
    el.className = 'admin-flash ' + (ok ? 'flash-ok' : 'flash-err');
    el.textContent = msg;
    setTimeout(function(){ el.textContent = ''; el.className = ''; }, 4000);
  }

  function formData(form) {
    return {
      name: form.name.value.trim(),
      host: form.host.value.trim(),
      category: form.category.value,
      icon_path: form.icon_path.value.trim() || null,
      discord_link: form.discord_link.value.trim() || null,
      is_wip: form.is_wip.checked,
      polled: form.polled.checked
    };
  }

  function api(method, url, body) {
    return fetch(url, {
      method: method,
      headers: {'Content-Type':'application/json'},
      body: body ? JSON.stringify(body) : undefined
    }).then(function(r){
      if (r.ok) return r.json();
      return r.text().then(function(t){ throw new Error(t); });
    });
  }

  // list page
  var rows = document.getElementById('server-rows');
  if (rows && Array.isArray(data)) {
    function renderRows() {
      var html = '';
      for (var i = 0; i < data.length; i++) {
        var s = data[i];
        html += '<tr>'
          + '<td>' + s.id + '</td>'
          + '<td>' + esc(s.name) + '</td>'
          + '<td>' + esc(s.category) + '</td>'
          + '<td class="admin-host">' + esc(s.host) + '</td>'
          + '<td>' + (s.polled ? 'Yes' : 'No') + '</td>'
          + '<td>' + (s.is_wip ? 'Yes' : 'No') + '</td>'
          + '<td>'
            + '<a href="/admin/edit/' + s.id + '">Edit</a> '
            + '<button class="btn-delete" data-id="' + s.id + '" data-name="' + esc(s.name) + '">Delete</button>'
          + '</td></tr>';
      }
      rows.innerHTML = html;
    }
    renderRows();

    rows.addEventListener('click', function(e) {
      var btn = e.target;
      if (!btn.classList.contains('btn-delete')) return;
      var id = btn.getAttribute('data-id');
      var name = btn.getAttribute('data-name');
      if (!confirm('Delete "' + name + '"? This removes all poll history.')) return;
      api('DELETE', '/api/admin/servers/' + id)
        .then(function(){ data = data.filter(function(s){ return String(s.id) !== id; }); renderRows(); flash('Deleted', true); })
        .catch(function(e){ flash('Delete failed: ' + e.message, false); });
    });

    var addForm = document.getElementById('add-form');
    if (addForm) addForm.addEventListener('submit', function(e) {
      e.preventDefault();
      api('POST', '/api/admin/servers', formData(addForm))
        .then(function(s){ data.push(s); renderRows(); addForm.reset(); flash('Created "' + s.name + '"', true); })
        .catch(function(e){ flash('Create failed: ' + e.message, false); });
    });
  }

  // edit page
  var editForm = document.getElementById('edit-form');
  if (editForm && data && data.id) {
    editForm.name.value = data.name || '';
    editForm.host.value = data.host || '';
    editForm.category.value = data.category || 'pserver';
    editForm.icon_path.value = data.icon_path || '';
    editForm.discord_link.value = data.discord_link || '';
    editForm.is_wip.checked = !!data.is_wip;
    editForm.polled.checked = !!data.polled;
    document.getElementById('edit-id').value = data.id;

    editForm.addEventListener('submit', function(e) {
      e.preventDefault();
      var id = document.getElementById('edit-id').value;
      api('PUT', '/api/admin/servers/' + id, formData(editForm))
        .then(function(){ flash('Saved', true); setTimeout(function(){ location.href = '/admin'; }, 800); })
        .catch(function(e){ flash('Save failed: ' + e.message, false); });
    });
  }
})();
"#;

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

  // keep in sync with uptime_color() in crates/app/src/uptime.rs
  function uptimeColor(p) {
    if (p >= 75) {
      var g = Math.floor(((p - 75) / 25) * 255);
      return 'rgb(' + (255 - g) + ', 255, 0)';
    }
    if (p >= 50) {
      var b = Math.floor(((p - 50) / 25) * 255);
      return 'rgb(255, 255, ' + b + ')';
    }
    if (p > 0) {
      var g = Math.floor((p / 50) * 255);
      return 'rgb(255, ' + g + ', 0)';
    }
    return 'rgb(255, 0, 0)';
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
  var SPARK_W = 120, SPARK_H = 24;
  function svgSparkline(points) {
    var w = SPARK_W, h = SPARK_H;
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
  function safeUrl(s){ s=String(s==null?'':s).trim(); if(s===''||s[0]==='/'||s.lastIndexOf('https://',0)===0||s.lastIndexOf('http://',0)===0) return s; return ''; }
  function renderCard(s) {
    var status = s.status;
    var isWip = status === 'wip';
    var statusText = isWip ? 'WIP' : (status === 'online' ? 'Online' : 'Offline');
    var linkText = (s.link||'').indexOf('discord') >= 0 ? 'Join Discord' : 'Visit Homepage';
    var players = isWip ? '-' : String(s.current_players);
    var peak = isWip ? '-' : String(s.peak_24h);
    var insecureBadge = s.secure ? '' : '<span class="insecure-badge" title="This server has a non-HTTPS API in 2026">&#9888; HTTP</span>';
    var iconUrl = safeUrl(s.icon_path);
    var linkUrl = safeUrl(s.link);
    var html = '<div class="server-card' + (s.secure ? '' : ' insecure') + '" data-server-id="'+s.id+'">'
      + '<div class="card-header">'
        + '<img src="'+esc(iconUrl)+'" alt="'+esc(s.name)+'" class="server-icon" data-discord="'+esc(s.link)+'"/>'
        + '<div class="server-info">'
          + '<h3 class="server-name">'+esc(s.name)+insecureBadge+'</h3>'
          + '<a href="'+esc(linkUrl)+'" class="server-discord" target="_blank" rel="noopener noreferrer">'+linkText+'</a>'
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
  renderGrid();
})();
"#;
