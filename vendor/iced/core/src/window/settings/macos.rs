//! Platform specific settings for macOS.

/// The platform specific window settings of an application.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlatformSpecific {
    /// Hides the window title.
    pub title_hidden: bool,
    /// Makes the titlebar transparent and allows the content to appear behind it.
    pub titlebar_transparent: bool,
    /// Makes the window content appear behind the titlebar.
    pub fullsize_content_view: bool,
    /// Groups windows together by using the same tabbing identifier
    /// (`NSWindow.tabbingIdentifier`). `None` leaves the system default
    /// (which may auto-tab this window with others from the same app, per
    /// the user's System Settings); `Some` values that differ from every
    /// other open window's identifier keep it in its own tab group, i.e.
    /// a genuinely separate window rather than a tab.
    pub tabbing_identifier: Option<String>,
}
