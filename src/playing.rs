mod access_token;

use actix_web::{HttpResponse, web};
use actix_web::body::BoxBody;
use actix_web::http::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use crate::{AppState, ErrorResponse};
use crate::playing::access_token::AccessToken;

#[derive(Serialize, Deserialize, Debug)]
pub struct Playing {
    album_name: String,
    album_artist: String,
    album_art: String,
    song_name: String,
}

impl Playing {
    pub async fn get_playing(state: web::Data<AppState>) -> HttpResponse {
        // Firstly determining if the access token we have can still be used.
        let current_time: u128 = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_millis();

        // Checking if it's empty.
        if state.access_token.lock().unwrap().is_empty() {
            // Access token is empty, so we need to make a new one.
            if let Ok(token) = AccessToken::get_access_token(String::from(state.client_tkn.clone() + ":" + &*state.client_secret.clone()), state.refresh_token.clone()).await {
                println!("Got a new access token: {}", token);
                *state.access_token.lock().unwrap() = token;
                *state.last_refresh.lock().unwrap() = current_time.to_string();
            }
        }else if current_time - state.last_refresh.lock().unwrap().parse::<u128>().unwrap() > 3600000 {
            // Access token can't be used, so we need to make a new one. Good god this pointer mess is awful
            if let Ok(token) = AccessToken::get_access_token(String::from(state.client_tkn.clone() + ":" + &*state.client_secret.clone()), state.refresh_token.clone()).await {
                *state.access_token.lock().unwrap() = token;
                *state.last_refresh.lock().unwrap() = current_time.to_string();
            }
        }

        let bearer_token = "Bearer ".to_owned() + &*state.access_token.lock().unwrap().replace('"', "");

        let client = reqwest::Client::new();
        let resp = client.get("https://api.spotify.com/v1/me/player/currently-playing?market=GB")
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, bearer_token)
            .send()
            .await;

        if let Ok(resp) = resp {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(item) = data["item"].as_object(){
                    if let Some(name) = item["name"].as_str(){
                        let song = Self {
                            album_name: item["album"]["name"].to_string().replace('"', ""),
                            album_artist: item["album"]["artists"][0]["name"].to_string().replace('"', ""),
                            album_art: item["album"]["images"][0]["url"].to_string().replace('"', ""),
                            song_name: name.to_string(),
                        };

                        let body = BoxBody::new(serde_json::to_string(&song).unwrap());
                        HttpResponse::Ok().content_type("application/json").body(body)
                    }else{
                        let body = BoxBody::new(serde_json::to_string(&ErrorResponse { message: "No song playing".to_string() }).unwrap());
                        HttpResponse::Ok().content_type("application/json").body(body)
                    }
                }else{
                    println!("{:?}", data);
                    let body = BoxBody::new(serde_json::to_string(&ErrorResponse { message: "No song playing".to_string() }).unwrap());
                    HttpResponse::Ok().content_type("application/json").body(body)
                }
            }else{
                let body = BoxBody::new(serde_json::to_string(&ErrorResponse { message: "No JSON Response from Spotify.".to_string() }).unwrap());
                HttpResponse::InternalServerError().content_type("application/json").body(body)
            }
        }else{
            let body = BoxBody::new(serde_json::to_string(&ErrorResponse { message: "No Response from Spotify.".to_string() }).unwrap());
            HttpResponse::InternalServerError().content_type("application/json").body(body)
        }
    }
}