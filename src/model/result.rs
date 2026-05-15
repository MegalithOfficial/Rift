#[derive(Clone)]
pub enum ResultAction {
    LaunchApp(gio::AppInfo),
    RunShell(String),
    CopyText(String),
}

#[derive(Clone)]
pub struct SearchResult {
    title: String,
    subtitle: String,
    executable: String,
    icon: Option<gio::Icon>,
    fallback_icon_name: &'static str,
    action: ResultAction,
    usage_key: String,
}

impl SearchResult {
    pub fn new(
        title: String,
        subtitle: String,
        executable: String,
        icon: Option<gio::Icon>,
        fallback_icon_name: &'static str,
        usage_key: String,
        action: ResultAction,
    ) -> Self {
        Self {
            title,
            subtitle,
            executable,
            icon,
            fallback_icon_name,
            action,
            usage_key,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn subtitle(&self) -> &str {
        &self.subtitle
    }

    pub fn executable(&self) -> &str {
        &self.executable
    }

    pub fn icon(&self) -> Option<&gio::Icon> {
        self.icon.as_ref()
    }

    pub fn fallback_icon_name(&self) -> &'static str {
        self.fallback_icon_name
    }

    pub fn action(&self) -> &ResultAction {
        &self.action
    }

    pub fn usage_key(&self) -> &str {
        &self.usage_key
    }
}
