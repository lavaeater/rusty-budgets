use dioxus::prelude::*;

const ECHO_CSS: Asset = asset!("/assets/styling/echo.css");

/// Echo component that demonstrates fullstack server functions.
#[component]
pub fn Users() -> Element {
    let users = use_resource(api::list_users).suspend()?;

    let users_2 = use_server_future(|| api::list_users())?;
    let users_2 = users_2().unwrap();
    rsx! {
        document::Link { rel: "stylesheet", href: ECHO_CSS }
        div {
            id: "echo",
            h4 { "Users, bro" }
            ul {
                    for user in users().unwrap().iter() {
                        li { key: "{user.id}", "{user.first_name} {user.last_name}" }
                    }
            }
            ul {
                for user in users_2.unwrap().iter() {
                    li { key: "{user.id}", "{user.first_name} {user.last_name}" }
                }
            }
        }
    }
}

