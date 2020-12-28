use rocket::Route;

mod tracks;
mod pps;


pub fn get_routes() -> Vec<Route> {
    let mut routes = Vec::new();

    let r1 = tracks::get_routes();
    let r2 = pps::get_routes();

    routes.extend(r1);
    routes.extend(r2);

    routes
}