use actix_web::http::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
#[derive(Deserialize)]
pub struct AccessToken {
    token: String,
}

impl AccessToken {
    pub async fn get_access_token(auth_header: String, refresh_token: String) -> Result<String, Box<dyn std::error::Error>>{
        let client = reqwest::Client::new();
        let res = client.post("https://accounts.spotify.com/api/token")
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(AUTHORIZATION, "Basic ".to_owned() + base64::encode(auth_header.clone()).as_str())
            .form(&[("grant_type", "refresh_token"), ("refresh_token", refresh_token.as_str())])
            .send()
            .await?;

        if let Ok(data) = res.json::<serde_json::Value>().await {
            if let Some(token) = data["access_token"].as_str(){
                Ok(data["access_token"].to_string())
            }else{
                println!("{:?}", data);
                Ok(String::new())
            }
        }else{
            Ok(String::new())
        }
    }

}