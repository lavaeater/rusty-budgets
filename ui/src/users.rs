use dioxus::prelude::*;

const ECHO_CSS: Asset = asset!("/assets/styling/echo.css");


#[component]
pub fn Users() -> Element {
    let users = use_server_future(|| api::list_users())?().unwrap().unwrap_or_default();
    rsx! {
        document::Link { rel: "stylesheet", href: ECHO_CSS }
        div {
            id: "users_list",
            h4 { "Users, bro" }
            table {
                thead {
                    tr {
                        th { "First Name" }
                        th { "Last Name" }
                    }
                }
                tbody {
                    for user in users.iter() {
                        tr {
                            key: "{user.id}",
                            td { "{user.first_name}" }
                            td { "{user.last_name}" }
                        }
                    }
                }
            }
        }
    }
}

