use crate::consts::DISCORD_API;
use log::warn;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
};
use reqwest::Client;
use rocket::http::private::cookie::Expiration;
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::response::{Flash, Redirect};
use rocket::uri;
use rocket::State;
use serenity::model::user::User;

#[get("/discord")]
pub async fn discord_login(
    oauth2_client: &State<BasicClient>,
    cookies: &CookieJar<'_>,
) -> Redirect {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = oauth2_client
        .authorize_url(CsrfToken::new_random)
        // Set the desired scopes.
        .add_scope(Scope::new("identify".to_string()))
        .add_scope(Scope::new("guilds".to_string()))
        .add_scope(Scope::new("email".to_string()))
        // Set the PKCE code challenge.
        .set_pkce_challenge(pkce_challenge)
        .url();

    // store the pkce secret to verify the authorization later
    cookies.add_private(
        Cookie::build("verify", pkce_verifier.secret().to_string())
            .http_only(true)
            .path("/login")
            .same_site(SameSite::Lax)
            .expires(Expiration::Session)
            .finish(),
    );

    // store the csrf token to verify no interference
    cookies.add_private(
        Cookie::build("csrf", csrf_token.secret().to_string())
            .http_only(true)
            .path("/login")
            .same_site(SameSite::Lax)
            .expires(Expiration::Session)
            .finish(),
    );

    Redirect::to(auth_url.to_string())
}

#[get("/discord/authorized?<code>&<state>")]
pub async fn discord_callback(
    code: &str,
    state: &str,
    cookies: &CookieJar<'_>,
    oauth2_client: &State<BasicClient>,
    reqwest_client: &State<Client>,
) -> Result<Redirect, Flash<Redirect>> {
    if let (Some(pkce_secret), Some(csrf_token)) =
        (cookies.get_private("verify"), cookies.get_private("csrf"))
    {
        if state == csrf_token.value() {
            let token_result = oauth2_client
                .exchange_code(AuthorizationCode::new(code.to_string()))
                // Set the PKCE code verifier.
                .set_pkce_verifier(PkceCodeVerifier::new(pkce_secret.value().to_string()))
                .request_async(async_http_client)
                .await;

            cookies.remove_private(Cookie::named("verify"));
            cookies.remove_private(Cookie::named("csrf"));

            match token_result {
                Ok(token) => {
                    cookies.add_private(
                        Cookie::build("access_token", token.access_token().secret().to_string())
                            .secure(true)
                            .http_only(true)
                            .path("/dashboard")
                            .finish(),
                    );

                    let request_res = reqwest_client
                        .get(format!("{}/users/@me", DISCORD_API))
                        .bearer_auth(token.access_token().secret())
                        .send()
                        .await;

                    match request_res {
                        Ok(response) => {
                            let user_res = response.json::<User>().await;

                            match user_res {
                                Ok(user) => {
                                    let user_name = format!("{}#{}", user.name, user.discriminator);
                                    let user_id = user.id.as_u64().to_string();

                                    cookies.add_private(Cookie::new("username", user_name));
                                    cookies.add_private(Cookie::new("userid", user_id));

                                    Ok(Redirect::to(uri!(super::return_to_same_site("dashboard"))))
                                }

                                Err(e) => {
                                    warn!("Error constructing user from request: {:?}", e);

                                    Err(Flash::new(
                                        Redirect::to(uri!(super::return_to_same_site(""))),
                                        "danger",
                                        "Failed to contact Discord",
                                    ))
                                }
                            }
                        }

                        Err(e) => {
                            warn!("Error getting user info: {:?}", e);

                            Err(Flash::new(
                                Redirect::to(uri!(super::return_to_same_site(""))),
                                "danger",
                                "Failed to contact Discord",
                            ))
                        }
                    }
                }

                Err(e) => {
                    warn!("Error in discord callback: {:?}", e);

                    Err(Flash::new(
                        Redirect::to(uri!(super::return_to_same_site(""))),
                        "warning",
                        "Your login request was rejected",
                    ))
                }
            }
        } else {
            Err(Flash::new(Redirect::to(uri!(super::return_to_same_site(""))), "danger", "Your request failed to validate, and so has been rejected (error: CSRF Validation Failure)"))
        }
    } else {
        Err(Flash::new(Redirect::to(uri!(super::return_to_same_site(""))), "warning", "Your request was missing information, and so has been rejected (error: CSRF Validation Tokens Missing)"))
    }
}
