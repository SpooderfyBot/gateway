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
}