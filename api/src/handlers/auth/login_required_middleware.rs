use oauth2::http::StatusCode;
use poem::{Endpoint, IntoResponse, Request, Response};
use poem::session::Session;
use entities::user;
use crate::handlers::auth::REDIRECT_AFTER_LOGIN_KEY;
use user::Model as User;

pub async fn login_required_middleware<E: Endpoint>(next: E, req: Request) -> poem::Result<Response> {
    let session = req.extensions().get::<Session>();

    if let Some(session) = session {
        // Check if user is logged in
        if session.get::<User>("current_user").is_some() {
            // User is logged in, proceed to the endpoint
            return match next.call(req).await {
                Ok(res) => Ok(res.into_response()),
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    Err(err)
                }
            };
        } else {
            session.set(REDIRECT_AFTER_LOGIN_KEY, req.uri().path().to_string());
        }
    }

    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", "/auth/login")
        .finish())
}