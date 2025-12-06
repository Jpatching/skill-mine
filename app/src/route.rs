use dioxus::prelude::*;

use crate::pages::{Home, Leaderboard, Play, Stats};
use crate::components::Layout;

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Layout)]
    #[route("/")]
    Play {},  // Game first - users see live game immediately
    #[route("/about")]
    Home {},  // Move landing page to /about
    #[route("/leaderboard")]
    Leaderboard {},
    #[route("/stats")]
    Stats {},
}
