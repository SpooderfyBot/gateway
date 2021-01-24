use serde::Serialize;

#[derive(Serialize)]
pub struct Message {
    pub icon_url: String,
    pub description: String,
    pub color: usize,
}

#[derive(Serialize)]
pub struct Wrapper<T> where T: Serialize {
    embeds: Vec<Message>,
    content: T
}

#[derive(Serialize)]
struct UserMessage {
    content: String,
    embeds: (),
    username: String,
    avatar_url: String,
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
        let embeds = Vec::from([payload]);
        let wrapped = Wrapper {
            embeds,
            content: ()
        };

        let res = self.client
            .post(&self.url)
            .json(&wrapped)
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