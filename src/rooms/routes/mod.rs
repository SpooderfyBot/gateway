use rocket::Route;

mod api;

pub fn get_routes() -> Vec<Route> {
    api::get_routes()
}

