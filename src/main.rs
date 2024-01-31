mod playing;

use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{Read};
use std::path::Path;
use std::sync::Mutex;
use actix_web::{web, App, HttpServer, HttpResponse};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use crate::playing::Playing;

#[derive(Serialize, Deserialize, Debug)]
struct ServerConfig {
    client_token: String,
    client_secret: String,
    refresh_token: String
}

#[derive(Serialize, Deserialize)]
struct ErrorResponse {
    message: String
}

struct AppState {
    app_name: String,
    client_tkn: String,
    client_secret: String,
    refresh_token: String,
    access_token: Mutex<String>,
    last_refresh: Mutex<String>,
}

#[derive(Deserialize)]
struct SpotifyCallbackResponse {
    code: String,
}

async fn process_refresh_token(client_token: String, client_secret: String, code: String) -> Result<String, Box<dyn std::error::Error>> {
    let auth: String = client_token + ":" + &client_secret;

    // Reqwest's documentation seems to suggest the best way to send off a POST request is by making a client.
    let client = reqwest::Client::new();
    let resp = client.post("https://accounts.spotify.com/api/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(AUTHORIZATION, "Basic ".to_owned() + base64::encode(auth.clone()).as_str())
        .form(&[("grant_type", "authorization_code"), ("code", code.as_str()), ("redirect_uri", "http://127.0.0.1:8080/callback")])
        .send()
        .await?;

    if let Ok(data) = resp.json::<serde_json::Value>().await {
        println!("Data: {:?}", data);

        if let Some(token) = data["refresh_token"].as_str(){
            // We have a token! Time to write this to the config file.
            let file = File::open("config.json");
            if let Ok(file) = file {
                let mut contents = String::new();

                file.take(1024).read_to_string(&mut contents).expect("TODO: panic message");

                if let Ok(mut data) = serde_json::from_str::<ServerConfig>(&contents) {
                    data.refresh_token = token.to_string();

                    let file = File::create("config.json");

                    if let Ok(file) = file {
                        if let Err(err) = serde_json::to_writer(file, &data) {
                            eprintln!("Error writing configuration template: {}", err);
                        }
                    } else {
                        eprintln!("Error editing configuration.");
                    }

                    Ok(("Successfully got a refresh token!".to_string()))
                } else {
                    eprintln!("Error deserializing JSON");
                    Ok("Error deserializing JSON".to_string())
                }
            }else{
                eprintln!("Couldn't open config JSON!");
                Ok("Couldn't open config JSON!".to_string())
            }
        }else{
            Ok("Didn't get an access token!".to_string())
        }
    }else{
        Ok("No JSON Response from inital request.".to_string())
    }
}

async fn spotify_account_callback(state: web::Data<AppState>, callback_code: web::Query<SpotifyCallbackResponse>) -> HttpResponse {
    // We have a callback code from Spotify. Now, we have to exchange it for a refresh token.

    let prc = process_refresh_token(state.client_tkn.clone(), state.client_secret.clone(), callback_code.code.clone()).await;
    println!("prc: {:?}", prc);
    HttpResponse::Ok().content_type("application/json").body("Hello World!")
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut tkn = String::from("");
    let mut secret = String::from("");
    let mut refresh = String::from("");

    if Path::new("config.json").exists() == false {
        println!("No server config found..");
        let template = ServerConfig {
            client_secret: String::from(""),
            client_token: String::from(""),
            refresh_token: String::from(""),
        };

        let file = File::create("config.json");

        if let Ok(file) = file {
            if let Err(err) = serde_json::to_writer(file, &template) {
                eprintln!("Error writing configuration template: {}", err);
            }
        }else{
            eprintln!("Error creating configuration template.");
        }
    }else{
        // Going to read the file, and check for any missing values.
        let file = File::open("config.json");
        if let Ok(file) = file {
            let mut contents = String::new();

            file.take(1024).read_to_string(&mut contents).expect("TODO: panic message");

            if let Ok(data) = serde_json::from_str::<ServerConfig>(&contents) {
                println!("Heres the data: {:?}", data);

                if data.client_token == "" {
                    println!("Missing Client Token!");
                }
                if data.client_secret == "" {
                    println!("Missing Client Secret!");
                }


                if data.refresh_token == "" {
                    let scope = ["user-read-currently-playing"];
                    let redirect_uri = "http://127.0.0.1:8080/callback";
                    let url = format!("https://accounts.spotify.com/authorize?client_id={}&response_type=code&redirect_uri={}&scope={}", data.client_token, redirect_uri, scope.join("%20"));

                    println!("Missing Refresh Token!");
                    println!("Please visit the URL below: \n{}",url);
                }

                tkn = data.client_token;
                secret = data.client_secret;
                refresh = data.refresh_token;


            }else{
                eprintln!("Error deserializing JSON");
            }
        }else{
            eprintln!("Couldn't open config JSON!");
        }
    }

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                app_name: String::from("Actix Web"),
                client_tkn: tkn.clone(),
                client_secret: secret.clone(),
                refresh_token: refresh.clone(),
                access_token: Mutex::new(String::from("")),
                last_refresh: Mutex::new(String::from("")),
            }))
            .route("/callback", web::get().to(spotify_account_callback))
            .route("/getplaying", web::get().to(Playing::get_playing))
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await


}