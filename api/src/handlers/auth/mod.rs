pub(crate) mod required_role_middleware;
pub(crate) mod login_required_middleware;

use openidconnect::core::{
    CoreAuthDisplay, CoreAuthPrompt, CoreClaimName, CoreClaimType, CoreClient,
    CoreClientAuthMethod, CoreGenderClaim, CoreGrantType, CoreIdTokenVerifier,
    CoreJsonWebKey, CoreJweContentEncryptionAlgorithm, CoreJweKeyManagementAlgorithm,
    CoreResponseMode, CoreResponseType, CoreSubjectIdentifierType, CoreTokenIntrospectionResponse,
    CoreTokenResponse,
};
use openidconnect::{
    AdditionalProviderMetadata, AuthenticationFlow, Client, ClientId, ClientSecret,
    EmptyAdditionalClaims, IssuerUrl, Nonce, ProviderMetadata, RedirectUrl, RevocationUrl,
    StandardErrorResponse,
};
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::AppState;
use entities::user::ActiveModel as UserActiveModel;
use oauth2::basic::{BasicErrorResponseType, BasicRevocationErrorResponse};
use oauth2::{
    AuthorizationCode, CsrfToken, EndpointMaybeSet, EndpointNotSet, EndpointSet, Scope,
    StandardRevocableToken,
};
use poem::error::NotFoundError;
use poem::http::StatusCode;
use poem::session::Session;
use poem::web::{Data, Redirect};
use poem::{get, handler, IntoResponse, Result, Route};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, NotSet};
use std::env;
use std::string::ToString;

use service::QueryCore as QueryCore;

// Teach openidconnect-rs about a Google custom extension to the OpenID Discovery response that we can use as the RFC
// 7009 OAuth 2.0 Token Revocation endpoint. For more information about the Google specific Discovery response see the
// Google OpenID Connect service documentation at: https://developers.google.com/identity/protocols/oauth2/openid-connect#discovery
#[derive(Clone, Debug, Deserialize, Serialize)]
struct RevocationEndpointProviderMetadata {
    revocation_endpoint: String,
}
impl AdditionalProviderMetadata for RevocationEndpointProviderMetadata {}
type GoogleProviderMetadata = ProviderMetadata<
    RevocationEndpointProviderMetadata,
    CoreAuthDisplay,
    CoreClientAuthMethod,
    CoreClaimName,
    CoreClaimType,
    CoreGrantType,
    CoreJweContentEncryptionAlgorithm,
    CoreJweKeyManagementAlgorithm,
    CoreJsonWebKey,
    CoreResponseMode,
    CoreResponseType,
    CoreSubjectIdentifierType,
>;

const REDIRECT_AFTER_LOGIN_KEY: &str = "redirect_after_login"; 
const NONCE_KEY: &str = "nonce";
pub type GoogleClient = Client<
    EmptyAdditionalClaims,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<BasicErrorResponseType>,
    CoreTokenResponse,
    CoreTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

fn handle_error<T: std::error::Error>(fail: &T, msg: &'static str) {
    let mut err_msg = format!("ERROR: {}", msg);
    let mut cur_fail: Option<&dyn std::error::Error> = Some(fail);
    while let Some(cause) = cur_fail {
        err_msg += &format!("\n    caused by: {}", cause);
        cur_fail = cause.source();
    }
    println!("{}", err_msg);
}

pub async fn setup_openid_client() -> anyhow::Result<GoogleClient> {
    let google_client_id = ClientId::new(
        env::var("GOOGLE_CLIENT_ID").expect("Missing the GOOGLE_CLIENT_ID environment variable."),
    );
    let google_client_secret = ClientSecret::new(
        env::var("GOOGLE_CLIENT_SECRET")
            .expect("Missing the GOOGLE_CLIENT_SECRET environment variable."),
    );
    let issuer_url =
        IssuerUrl::new("https://accounts.google.com".to_string()).unwrap_or_else(|err| {
            handle_error(&err, "Invalid issuer URL");
            unreachable!();
        });

    let auth_redirect_url = env::var("GOOGLE_REDIRECT_URI").unwrap_or_else(|_| "http://127.0.0.1:8000/auth/callback".to_string());
    
    let http_client = get_http_client();

    let provider_metadata = GoogleProviderMetadata::discover_async(issuer_url, &http_client)
        .await
        .unwrap_or_else(|err| {
            handle_error(&err, "Failed to discover OpenID Provider");
            unreachable!();
        });

    let revocation_endpoint = provider_metadata
        .additional_metadata()
        .revocation_endpoint
        .clone();
    println!(
        "Discovered Google revocation endpoint: {}",
        revocation_endpoint
    );

    // Set up the config for the Google OAuth2 process.
    let client: GoogleClient = CoreClient::from_provider_metadata(
        provider_metadata,
        google_client_id,
        Some(google_client_secret),
    )
    // This example will be running its own server at localhost:8080.
    // See below for the server implementation.
    .set_redirect_uri(
        RedirectUrl::new(auth_redirect_url).unwrap_or_else(|err| {
            handle_error(&err, "Invalid redirect URL");
            unreachable!();
        }),
    )
    // Google supports OAuth 2.0 Token Revocation (RFC-7009)
    .set_revocation_url(
        RevocationUrl::new(revocation_endpoint).unwrap_or_else(|err| {
            handle_error(&err, "Invalid revocation endpoint URL");
            unreachable!();
        }),
    );
    Ok(client)
}

fn get_http_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_else(|err| {
            handle_error(&err, "Failed to create HTTP client");
            unreachable!();
        })
}

#[handler]
async fn login(session: &Session, auth_client: Data<&GoogleClient>) -> Result<impl IntoResponse> {
    let (authorize_url, _csrf_state, nonce) = auth_client
        .0
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .url();
    
    session.set(NONCE_KEY, nonce);
    
    // Access-Control-Allow-Origin
    Ok(StatusCode::FOUND
        .with_header("HX-Redirect", authorize_url.to_string())
        .with_header("Access-Control-Allow-Origin", "*"))
}

#[handler]
async fn auth_callback(
    auth_client: Data<&GoogleClient>,
    app_state: Data<&AppState>,
    query: poem::web::Query<HashMap<String, String>>,
    session: &Session,
) -> Result<Redirect> {
    let code = query.get("code");
    if let Some(code) = code {
        let http_client = get_http_client();

        let token_response = auth_client
            .0
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .unwrap()
            .request_async(&http_client)
            .await
            .unwrap_or_else(|err| {
                handle_error(&err, "Failed to exchange code for token");
                unreachable!();
            });
        
        let nonce = session.get::<Nonce>(NONCE_KEY).unwrap();

        let id_token_verifier: CoreIdTokenVerifier = auth_client.id_token_verifier();
        match token_response
            .extra_fields()
            .id_token()
            .expect("Server did not return an ID token")
            .claims(
                &id_token_verifier,
                &nonce,
            ) {
            Ok(id_token_claims) => {
                if let Some(email) = id_token_claims.email() {
                    let boffo = QueryCore::find_user_by_email(&app_state.conn, email.as_str())
                        .await;

                    match boffo {
                        Ok(Some(user)) => {
                            session.set("current_user", user);
                        }
                        Err(err) => {
                            eprintln!("Error: {:?}", err);
                            return Err(NotFoundError.into());
                        },
                        Ok(None) => {
                            let u = UserActiveModel {
                                id: NotSet,
                                email: Set(email.to_string()),
                                name: Set("".to_string()),
                                role: NotSet,
                            }
                                .insert(&app_state.conn)
                                .await;
                            if let Ok(u) = u {
                                session.set("current_user", u);
                            }
                        }
                    }
                }

                return Ok(Redirect::temporary(session.get(REDIRECT_AFTER_LOGIN_KEY).unwrap_or("/".to_string())));
            }
            Err(err) => {
                println!("Failed to verify ID token: {}", &err);
            }
        } 
    }
    Err(NotFoundError.into())
}

// A function to define all routes related to posts
pub fn routes() -> Route {
    Route::new()
        .at("/login", get(login))
        .at("/callback", get(auth_callback))
}
