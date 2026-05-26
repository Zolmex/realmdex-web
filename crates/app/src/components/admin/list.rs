use leptos::prelude::*;

use crate::types::AdminServerRow;

#[component]
pub fn ListPage(email: String, #[allow(unused)] servers: Vec<AdminServerRow>) -> impl IntoView {
    view! {
        <div class="admin-container">
            <div class="admin-header">
                <h1>"RealmDex Admin"</h1>
                <span class="admin-user">{email}</span>
                <a href="/" class="admin-back">"Back to site"</a>
            </div>
            <div id="admin-flash"></div>
            <details class="admin-add-form">
                <summary>"Add New Server"</summary>
                <form id="add-form" class="admin-form">
                    <label>"Name "<input name="name" required maxlength="100"/></label>
                    <label>"Host URL "<input name="host" required placeholder="https://..."/></label>
                    <label>"Category"
                        <select name="category">
                            <option value="pserver">"Private Server"</option>
                            <option value="realm-like">"Realm-Like"</option>
                        </select>
                    </label>
                    <label>"Icon Path "<input name="icon_path" maxlength="500" placeholder="/content/images/..."/></label>
                    <label>"Discord Link "<input name="discord_link" maxlength="500" placeholder="https://discord.gg/..."/></label>
                    <label><input type="checkbox" name="is_wip"/>" Work in Progress"</label>
                    <label><input type="checkbox" name="polled" checked/>" Polled"</label>
                    <button type="submit">"Create Server"</button>
                </form>
            </details>
            <table class="admin-table">
                <thead><tr>
                    <th>"ID"</th><th>"Name"</th><th>"Category"</th><th>"Host"</th><th>"Polled"</th><th>"WIP"</th><th>"Actions"</th>
                </tr></thead>
                <tbody id="server-rows"></tbody>
            </table>
        </div>
    }
}
