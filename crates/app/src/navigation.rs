//! Page navigation and per-page scroll positions.
//!
//! Navigation is per top-level [`Section`]: each section keeps its own back stack, so switching
//! sections (toolbar tabs / the download button) restores where you were in each one instead of
//! sharing a single global history. Sub-pages belong to whichever section they were opened from.

use crate::prelude::*;
use crate::shell::state::{HISTORY_LIMIT, MikanPlus};

impl MikanPlus {
    /// Switch to a top-level section, restoring that section's own position (or its root page on
    /// first entry).
    pub(crate) fn switch_section(&mut self, section: Section, cx: &mut Context<Self>) {
        self.section = section;
        let root = section.root_page();
        let stack = self.stacks.entry(section).or_insert_with(|| vec![root]);
        let page = stack.last().cloned().expect("section stack is non-empty");
        self.enter_page(page, cx);
    }

    /// Navigate to a page, pushing it onto the active section's stack. A page that is another
    /// section's root is a section switch: that section's own position is restored.
    pub(crate) fn navigate_to(&mut self, page: Page, cx: &mut Context<Self>) {
        let section = Section::from_page(&page).unwrap_or(self.section);
        if section != self.section {
            self.switch_section(section, cx);
            return;
        }
        let stack = self
            .stacks
            .get_mut(&section)
            .expect("the active section has a stack");
        if stack.last() != Some(&page) {
            stack.push(page.clone());
            // Bound the back stack: drop the oldest entry when the limit is exceeded
            if stack.len() > HISTORY_LIMIT {
                stack.remove(0);
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
        let stack = self
            .stacks
            .get_mut(&self.section)
            .expect("the active section has a stack");
        if stack.len() > 1 {
            stack.pop();
            self.page = stack.last().cloned().expect("section stack is non-empty");
            cx.notify();
        }
    }

    /// Whether the active section has a page to go back to.
    pub(crate) fn can_go_back(&self) -> bool {
        self.stacks
            .get(&self.section)
            .is_some_and(|stack| stack.len() > 1)
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
