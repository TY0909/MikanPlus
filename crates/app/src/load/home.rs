//! Home data loading.

use crate::data::model::{LoadUpdate, Loadable, publish};
use crate::prelude::*;
use crate::shell::state::MikanPlus;

impl MikanPlus {
    /// Load the home page once; a session hit lives in [`AppData`](crate::data::model::AppData), so this only runs on startup and retry.
    pub(crate) fn load_home(&mut self, cx: &mut Context<Self>) {
        if self.data.home.is_loading() {
            return;
        }
        // User-initiated load / retry: clear this page's backoff state and proceed immediately
        self.network
            .reset_backoff(&source::api::home_url(&self.network));
        let network = self.network.clone();
        self.data.home = Loadable::Loading;
        cx.notify();
        std::thread::spawn(move || {
            let result = source::api::fetch_home(&network);
            publish(LoadUpdate::Home(result));
        });
    }
}
