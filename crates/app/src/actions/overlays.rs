//! Operations on overlay (modal window) state.

use crate::prelude::*;
use crate::shell::state::MikanPlus;
use domain::SubgroupRef;

impl MikanPlus {
    /// Open the episode filter window (prefilled with the current keyword).
    pub(crate) fn open_filter(&mut self, key: SubgroupRef, cx: &mut Context<Self>) {
        let keyword = self.data.keywords.get(&key).cloned().unwrap_or_default();
        let handle = self.window_handle;
        let input = self.filter_input.clone();
        let _ = handle.update(cx, move |_, window, cx: &mut App| {
            input.update(cx, |state, cx| state.set_value(keyword, window, cx));
        });
        self.overlays.filter = Some(key);
        cx.notify();
    }

    /// Read the input box and apply it as the current subscription entry's filter keyword (empty = clear), then close the window.
    pub(crate) fn apply_filter(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.overlays.filter else {
            return;
        };
        let text = self.filter_input.read(cx).text().to_string();
        let text = text.trim().to_string();
        if text.is_empty() {
            self.data.keywords.remove(&key);
        } else {
            self.data.keywords.insert(key, text);
        }
        self.state.set_subgroup_keywords(&self.data.keywords);
        self.overlays.filter = None;
        cx.notify();
    }

    /// Clear the current subscription entry's filter keyword and close the window.
    pub(crate) fn clear_filter(&mut self, cx: &mut Context<Self>) {
        if let Some(key) = self.overlays.filter {
            self.data.keywords.remove(&key);
        }
        self.state.set_subgroup_keywords(&self.data.keywords);
        self.overlays.filter = None;
        cx.notify();
    }

    /// Close the filter window (without changing the keyword).
    pub(crate) fn close_filter(&mut self, cx: &mut Context<Self>) {
        self.overlays.filter = None;
        cx.notify();
    }

    /// Close the unsubscribe warning window.
    pub(crate) fn close_warning(&mut self, cx: &mut Context<Self>) {
        self.overlays.warning = None;
        cx.notify();
    }

    /// Close the unsubscribe confirmation window (cancels the unsubscribe).
    pub(crate) fn close_confirmation(&mut self, cx: &mut Context<Self>) {
        self.overlays.confirmation = None;
        cx.notify();
    }

    /// Update the temporary cleanup option in the unsubscribe confirmation window.
    pub(crate) fn set_remove_downloads(&mut self, remove: bool, cx: &mut Context<Self>) {
        if let Some(confirmation) = self.overlays.confirmation.as_mut() {
            confirmation.remove_downloads = remove;
            cx.notify();
        }
    }
}
