use leptos::prelude::*;

use crate::types::AdminServerRow;

#[component]
pub fn EditPage(email: String, #[allow(unused)] server: AdminServerRow) -> impl IntoView {
    view! {
        <div class="admin-container">
            <div class="admin-header">
                <h1>"RealmDex Admin — Edit Server"</h1>
                <span class="admin-user">{email}</span>
                <a href="/admin" class="admin-back">"Back to list"</a>
            </div>
            <div id="admin-flash"></div>
            <form id="edit-form" class="admin-form">
                <input type="hidden" name="id" id="edit-id"/>
                <label>"Name "<input name="name" required maxlength="100"/></label>
                <label>"Host URL "<input name="host" required/></label>
                <label>"Category"
                    <select name="category">
                        <option value="pserver">"Private Server"</option>
                        <option value="realm-like">"Realm-Like"</option>
                    </select>
                </label>
                <label>"Icon Path "<input name="icon_path" maxlength="500"/></label>
                <label>"Discord Link "<input name="discord_link" maxlength="500"/></label>
                <label><input type="checkbox" name="is_wip"/>" Work in Progress"</label>
                <label><input type="checkbox" name="polled"/>" Polled"</label>
                <button type="submit">"Save Changes"</button>
            </form>
        </div>
    }
}
