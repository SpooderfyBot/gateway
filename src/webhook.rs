use serde::Serialize;

#[derive(Serialize)]
pub struct Message {
    username: String,
    avatar_url: String,
    content: String,
}

pub struct Webhook {
    client: reqwest::Client,
    url: String,
}

impl Webhook {
    pub fn new(url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            url
        }
    }

    pub async fn send(&self, payload: Message) -> Result<bool, reqwest::Error> {
        let res = self.client
            .post(&self.url)
            .json(&payload)
            .send()
            .await?;

        Ok(res.status().as_u16() < 400)
    }
}