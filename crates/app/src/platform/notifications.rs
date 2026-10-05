//! App-level toast notifications (download failures / completions, engine failures).

use gpui_kit::component::notification::Notification;

use crate::prelude::*;
use crate::shell::state::MikanPlus;

impl MikanPlus {
    /// Push an error notification in the top-right corner of the app.
    pub(crate) fn notify_error(&self, message: String, cx: &mut Context<Self>) {
        let _ = self.window_handle.update(cx, |_, window, cx| {
            window.push_notification(Notification::error(message), cx);
        });
    }

    /// Push a success notification in the top-right corner of the app.
    pub(crate) fn notify_success(&self, message: String, cx: &mut Context<Self>) {
        let _ = self.window_handle.update(cx, |_, window, cx| {
            window.push_notification(Notification::success(message), cx);
        });
    }
}
