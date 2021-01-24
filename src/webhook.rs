use serde::Serialize;


#[derive(Serialize)]
pub struct UserMessage {
    pub content: String,
    pub embeds: (),
    pub username: String,
    pub avatar_url: String,
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

    pub async fn send(&self, payload: UserMessage) -> Result<bool, reqwest::Error> {
        let res = self.client
            .post(&self.url)
            .json(&payload)
            .send()
            .await?;

        Ok(res.status().as_u16() < 400)
    }

    pub async fn send_as_user(
        &self,
        username: String,
        user_icon: String,
        message: String,
    ) -> Result<bool, reqwest::Error> {
        let msg = UserMessage {
            content: message,
            embeds: (),
            username,
            avatar_url: user_icon
        };

        let res = self.client
            .post(&self.url)
            .json(&msg)
            .send()
            .await?;

        Ok(res.status().as_u16() < 400)
    }
}