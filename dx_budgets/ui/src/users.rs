use dioxus::prelude::*;

const ECHO_CSS: Asset = asset!("/assets/styling/echo.css");

#[component]
pub fn Users() -> Element {
    let users_2 = use_server_future(|| api::list_users())?().unwrap().unwrap_or_default();
    rsx! {
        document::Link { rel: "stylesheet", href: ECHO_CSS }
        div {
            id: "echo",
            h4 { "Users, bro" }
            ul {
                for user in users_2.iter() {
                    li { key: "{user.id}", "{user.first_name} {user.last_name}" }
                }
            }
        }
    }
}

