// //!
// //! This example showcases the process of integrating with the
// //! [Google OpenID Connect](https://developers.google.com/identity/protocols/OpenIDConnect)
// //! provider.
// //!
// //! Before running it, you'll need to generate your own Google OAuth2 credentials.
// //!
// //! In order to run the example call:
// //!
// //! ```sh
// //! GOOGLE_CLIENT_ID=xxx GOOGLE_CLIENT_SECRET=yyy cargo run --example google
// //! ```
// //!
// //! ...and follow the instructions.
// //!
// 
// 
// pub async fn fjonk_e_bonke() {
//     // get env vars
//     env_logger::init();
// 
//     
// 
//     // Generate the authorization URL to which we'll redirect the user.
//     let (authorize_url, csrf_state, nonce) = client
//         .authorize_url(
//             AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
//             CsrfToken::new_random,
//             Nonce::new_random,
//         )
//         // This example is requesting access to the "calendar" features and the user's profile.
//         .add_scope(Scope::new("email".to_string()))
//         .add_scope(Scope::new("profile".to_string()))
//         .url();
// 
//     println!("Open this URL in your browser:\n{}\n", authorize_url);
// 
//     let (code, state) = {
//         // A very naive implementation of the redirect server.
//         let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
// 
//         // Accept one connection
//         let (mut stream, _) = listener.accept().unwrap();
// 
//         let mut reader = BufReader::new(&stream);
// 
//         let mut request_line = String::new();
//         reader.read_line(&mut request_line).unwrap();
// 
//         let redirect_url = request_line.split_whitespace().nth(1).unwrap();
//         let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();
// 
//         let code = url
//             .query_pairs()
//             .find(|(key, _)| key == "code")
//             .map(|(_, code)| AuthorizationCode::new(code.into_owned()))
//             .unwrap();
// 
//         let state = url
//             .query_pairs()
//             .find(|(key, _)| key == "state")
//             .map(|(_, state)| CsrfToken::new(state.into_owned()))
//             .unwrap();
// 
//         let message = "Go back to your terminal :)";
//         let response = format!(
//             "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
//             message.len(),
//             message
//         );
//         stream.write_all(response.as_bytes()).unwrap();
// 
//         (code, state)
//     };
// 
//     println!("Google returned the following code:\n{}\n", code.secret());
//     println!(
//         "Google returned the following state:\n{} (expected `{}`)\n",
//         state.secret(),
//         csrf_state.secret()
//     );
// 
//     // Exchange the code with a token.
//     let token_response = client
//         .exchange_code(code)
//         .unwrap_or_else(|err| {
//             handle_error(&err, "No user info endpoint");
//             unreachable!();
//         })
//         .request_async(&http_client)
//         .await
//         .unwrap_or_else(|err| {
//             handle_error(&err, "Failed to contact token endpoint");
//             unreachable!();
//         });
// 
//     println!(
//         "Google returned access token:\n{}\n",
//         token_response.access_token().secret()
//     );
//     println!("Google returned scopes: {:?}", token_response.scopes());
// 
//     let id_token_verifier: CoreIdTokenVerifier = client.id_token_verifier();
//     let id_token_claims: &CoreIdTokenClaims = token_response
//         .extra_fields()
//         .id_token()
//         .expect("Server did not return an ID token")
//         .claims(&id_token_verifier, &nonce)
//         .unwrap_or_else(|err| {
//             handle_error(&err, "Failed to verify ID token");
//             unreachable!();
//         });
//     println!("Google returned ID token: {:?}", id_token_claims);
// 
//     // Revoke the obtained token
//     let token_to_revoke: CoreRevocableToken = match token_response.refresh_token() {
//         Some(token) => token.into(),
//         None => token_response.access_token().into(),
//     };
// 
//     client
//         .revoke_token(token_to_revoke)
//         .unwrap_or_else(|err| {
//             handle_error(&err, "Failed to revoke token");
//             unreachable!();
//         })
//         .request_async(&http_client)
//         .await
//         .unwrap_or_else(|err| {
//             handle_error(&err, "Failed to revoke token");
//             unreachable!();
//         });
// }