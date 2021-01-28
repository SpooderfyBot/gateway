use rocket::{Route, State};

use serde::Deserialize;

use crate::clients::{User, Sessions};
use crate::json::Json;

#[derive(Deserialize)]
struct AddSession {
    session_id: String,
    user: User,

}

#[post("/sessions/add", data="<session>")]
async fn add_session(session: Json<AddSession>, sessions: State<'_, Sessions>) {
    let sess = session.into_inner();
    sessions.add_user(sess.session_id, sess.user).await;
}

pub fn get_routes() -> Vec<Route> {
    routes![add_session]
}