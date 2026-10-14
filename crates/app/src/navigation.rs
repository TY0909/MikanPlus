//! Page navigation and per-page scroll positions.
//!
//! Navigation is per top-level [`Section`]: each section keeps its own root page and temporary
//! child history. Switching to another section restores that section's current page, while an
//! explicit home action can return to its root page. Sub-pages belong to whichever section they
//! were opened from.

use crate::prelude::*;
use crate::shell::state::{HISTORY_LIMIT, MikanPlus, SectionNavigation};

impl MikanPlus {
    /// Switch to a top-level section.
    ///
    /// Switching to another section restores its current page, including a detail/search page.
    /// Selecting the already-active section is treated as a request to return to that section's
    /// root page, which is how the logo returns home from a detail page.
    pub(crate) fn switch_section(&mut self, section: Section, cx: &mut Context<Self>) {
        let page = if section == self.section {
            self.sections
                .get_mut(&section)
                .expect("the active section has navigation state")
                .return_to_root()
        } else {
            self.activate_section(section, None)
        };
        self.enter_page(page, cx);
    }

    /// Navigate to a page, pushing it onto the active section's stack. A page that is another
    /// section's root is a section switch: that section's root state is restored.
    pub(crate) fn navigate_to(&mut self, page: Page, cx: &mut Context<Self>) {
        if let Some(section) = Section::from_page(&page) {
            if section != self.section {
                let page = self.activate_section(section, Some(page));
                self.enter_page(page, cx);
                return;
            }

            let page = {
                let state = self
                    .sections
                    .get_mut(&section)
                    .expect("the active section has navigation state");
                state.root = page;
                state.history.clear();
                state.root.clone()
            };
            self.enter_page(page, cx);
            return;
        }

        let state = self
            .sections
            .get_mut(&self.section)
            .expect("the active section has navigation state");
        if state.history.last() != Some(&page) {
            state.history.push(page.clone());
            // Bound the back stack: drop the oldest entry when the limit is exceeded
            if state.history.len() > HISTORY_LIMIT {
                state.history.remove(0);
            }
        }
        self.enter_page(page, cx);
    }

    /// Submit a keyword from the search page input box.
    pub(crate) fn submit_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).text().to_string();
        let query = query.trim().to_string();
        if !query.is_empty() {
            self.navigate_to(Page::SearchResult(query), cx);
        }
    }

    /// Return to the previous page in the active section's stack, if any.
    pub(crate) fn go_back(&mut self, cx: &mut Context<Self>) {
        let page = {
            let state = self
                .sections
                .get_mut(&self.section)
                .expect("the active section has navigation state");
            state.history.pop();
            state
                .history
                .last()
                .cloned()
                .unwrap_or_else(|| state.root.clone())
        };
        if page != self.page {
            self.enter_page(page, cx);
        }
    }

    /// Whether the active section has a page to go back to.
    pub(crate) fn can_go_back(&self) -> bool {
        self.sections
            .get(&self.section)
            .is_some_and(|state| !state.history.is_empty())
    }

    /// Activate a section and return the page that should be displayed.
    ///
    /// A root override is used when navigation explicitly targets a root page, such as the
    /// subscription empty-state button navigating to today's home page. Toolbar navigation omits
    /// it so the section's existing current page is preserved.
    fn activate_section(&mut self, section: Section, root: Option<Page>) -> Page {
        let default_root = section.root_page();
        let state = self
            .sections
            .entry(section)
            .or_insert_with(|| SectionNavigation::new(default_root));
        if let Some(root) = root {
            state.root = root;
            state.history.clear();
        }
        self.section = section;
        state.current_page()
    }

    /// The current top-level section, or `None` on a sub-page.
    pub(crate) fn current_section(&self) -> Option<Section> {
        Section::from_page(&self.page)
    }

    /// Enter a page: close overlays bound to the previous page, trigger on-demand loads, and redraw.
    fn enter_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page.clone();
        // Close overlays bound to the old page when switching pages
        self.overlays.filter = None;
        self.overlays.warning = None;
        self.overlays.confirmation = None;
        self.overlays.checking = None;
        // Detail / search pages: load on demand on entry (reused on a hit)
        match &page {
            Page::BangumiDetail { bangumi_id, name } => {
                self.load_detail(*bangumi_id, name.clone(), cx)
            }
            Page::SearchResult(query) if !query.trim().is_empty() => {
                self.load_search(query.clone(), cx);
            }
            _ => {}
        }
        cx.notify();
    }

    /// Toolbar title (breadcrumb).
    pub(crate) fn page_title(&self) -> String {
        match &self.page {
            Page::Home(filter) => filter.label().to_string(),
            Page::Subscription => "我的订阅".to_string(),
            Page::Settings => "设置".to_string(),
            Page::BangumiDetail { name, .. } => name.clone(),
            Page::SubGroupDetail(_) => "字幕组".to_string(),
            Page::SearchResult(query) if query.trim().is_empty() => "搜索番剧".to_string(),
            Page::SearchResult(query) => format!("搜索「{query}」"),
            Page::DownloadObserver => "下载观测".to_string(),
            Page::DownloadCollection(_) => "查看合集".to_string(),
        }
    }

    /// Scroll state key for the current page (different pages / filters keep independent scroll positions).
    pub(crate) fn scroll_key(&self) -> String {
        match &self.page {
            Page::Home(filter) => format!("home:{filter:?}"),
            Page::Subscription => "subscription".to_string(),
            Page::Settings => "settings".to_string(),
            Page::BangumiDetail { name, .. } => format!("detail:{name}"),
            Page::SubGroupDetail(key) => {
                format!("subgroup:{}:{}", key.bangumi.get(), key.subgroup.get())
            }
            Page::SearchResult(query) => format!("search:{query}"),
            Page::DownloadObserver => "download-observer".to_string(),
            Page::DownloadCollection(id) => format!("download-collection:{id}"),
        }
    }

    /// Get (or create) the page's scroll handle so the position is restored when re-entering the page.
    pub(crate) fn scroll_handle(&mut self, key: String) -> ScrollHandle {
        if let Some(handle) = self.scroll.get(&key) {
            return handle.clone();
        }
        let handle = ScrollHandle::new();
        self.scroll.insert(key, handle.clone());
        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_navigation_keeps_root_separate_from_temporary_history() {
        let mut state = SectionNavigation::new(Page::Home(HomeFilter::Weekday(0)));
        let detail = Page::SearchResult("凡人".to_string());
        state.history.push(detail.clone());

        assert_eq!(state.root, Page::Home(HomeFilter::Weekday(0)));
        assert_eq!(state.current_page(), detail);
        assert_eq!(state.return_to_root(), Page::Home(HomeFilter::Weekday(0)));
        assert!(state.history.is_empty());
    }
}
