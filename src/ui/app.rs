// Main TUI application
use crate::auth::{AuthManager, DeviceAuthorizationInfo};
use crate::cache::{self, CachedProfile, ProfileCache};
use crate::credentials::CredentialManager;
use crate::error::{Result, SsoError};
use crate::models::{AccountRole, SsoInstance, SsoToken};
use crate::sso_config;
use catppuccin::Flavor;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table, TableState,
    },
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io;
use tokio::sync::mpsc;

// Re-export types from extracted modules
use super::state::{
    AppState, DefaultsConfigStep, NewProfileConfigStep, SsoConfigStep, StaticCredentialStep,
};
use super::theme::catppuccin_color;
use super::types::{
    AccountRoleWithStatus, ActivePane, ConfirmAction, LoginResult, ProfileEntry, SsoSessionInfo,
};

/// Sort order for SSM browser instance list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SsmSortOrder {
    /// No sorting (default order from API)
    None,
    /// Sort by instance name (alphabetical)
    Name,
    /// Sort by instance ID (alphabetical)
    InstanceId,
    /// Sort by private IP address (alphabetical)
    PrivateIp,
}

impl SsmSortOrder {
    fn next(&self) -> Self {
        match self {
            Self::None => Self::Name,
            Self::Name => Self::InstanceId,
            Self::InstanceId => Self::PrivateIp,
            Self::PrivateIp => Self::None,
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::None => "unsorted",
            Self::Name => "name",
            Self::InstanceId => "ID",
            Self::PrivateIp => "IP",
        }
    }
}

pub struct App {
    /// Whether the app should quit
    should_quit: bool,
    /// Current screen/state
    state: AppState,
    /// Active pane (Sessions or Accounts)
    active_pane: ActivePane,
    /// List of SSO sessions with their status
    sso_sessions: Vec<SsoSessionInfo>,
    /// SSO sessions table selection state
    sessions_list_state: TableState,
    /// List of profiles (both SSO and static) with their active status
    accounts: Vec<ProfileEntry>,
    /// Accounts table selection state
    accounts_list_state: TableState,
    /// Cache of accounts/roles per SSO session (session_name -> accounts)
    accounts_cache: std::collections::HashMap<String, Vec<AccountRole>>,
    /// Current session filter (if Some, show only this session's accounts)
    filtered_session: Option<String>,
    /// Disk cache state (if Some, showing cached data with timestamp)
    showing_cached_data: Option<ProfileCache>,
    /// Authentication manager
    auth_manager: AuthManager,
    /// Credential manager
    credential_manager: CredentialManager,
    /// Current SSO instance (from selected session)
    sso_instance: Option<SsoInstance>,
    /// Current SSO token (from selected session)
    sso_token: Option<SsoToken>,
    /// Status message to display
    status_message: Option<String>,
    /// Profile name input buffer
    profile_input: String,
    /// Cursor position in profile input (0-based index)
    profile_input_cursor: usize,
    /// Account/role being configured
    pending_role: Option<AccountRole>,
    /// Existing profile name for pending role (if found)
    existing_profile_name: Option<String>,
    /// Device authorization info during login (updated via watch channel)
    device_auth_info: Option<DeviceAuthorizationInfo>,
    /// Watch channel receiver for device auth info from background login task
    device_auth_rx: Option<tokio::sync::watch::Receiver<Option<DeviceAuthorizationInfo>>>,
    /// Last Ctrl+C press time for double-press detection
    last_ctrl_c_time: Option<std::time::Instant>,
    /// Pending confirmation action (for modal dialog)
    pending_confirm_action: Option<ConfirmAction>,
    /// SSO configuration input buffers
    sso_start_url_input: String,
    sso_region_input: String,
    sso_session_name_input: String,
    sso_input_cursor: usize,
    /// Original session name when editing (None if creating new)
    editing_session_original_name: Option<String>,
    /// Default configuration input buffers
    default_region_input: String,
    default_output_input: String,
    default_input_cursor: usize,
    /// New profile configuration input buffers
    new_profile_name_input: String,
    new_profile_region_input: String,
    new_profile_output_input: String,
    new_profile_input_cursor: usize,
    /// Static credential input buffers
    static_profile_name_input: String,
    static_access_key_input: String,
    static_secret_key_input: String,
    static_session_token_input: String,
    static_input_cursor: usize,
    /// Last automatic refresh time
    last_auto_refresh: Option<std::time::Instant>,
    /// Catppuccin theme flavor
    theme: Flavor,
    /// Animation tick counter (increments each frame)
    tick_count: u64,
    /// Channel for receiving login results from background tasks
    login_rx: mpsc::UnboundedReceiver<LoginResult>,
    /// Sender for login tasks (kept to create clones for background tasks)
    login_tx: mpsc::UnboundedSender<LoginResult>,
    /// SSM browser: list of EC2 instances
    ssm_instances: Vec<crate::ssm::SsmInstance>,
    /// SSM browser: table selection state
    ssm_list_state: TableState,
    /// SSM browser: filter input
    ssm_filter: String,
    /// SSM browser: search mode (true when user is actively typing search)
    ssm_search_mode: bool,
    /// SSM browser: loading state
    ssm_loading: bool,
    /// SSM browser: show offline instances (per-session setting)
    ssm_show_offline: bool,
    /// SSM browser: current sort order
    ssm_sort_order: SsmSortOrder,
}

impl App {
    pub fn new() -> Result<Self> {
        let auth_manager = AuthManager::new()?;
        let credential_manager = CredentialManager::new()?;

        // Create channel for background login tasks
        let (login_tx, login_rx) = mpsc::unbounded_channel();

        Ok(Self {
            should_quit: false,
            state: AppState::Main,
            active_pane: ActivePane::Sessions,
            sso_sessions: Vec::new(),
            sessions_list_state: TableState::default(),
            accounts: Vec::new(),
            accounts_list_state: TableState::default(),
            accounts_cache: std::collections::HashMap::new(),
            filtered_session: None,
            showing_cached_data: None,
            auth_manager,
            credential_manager,
            sso_instance: None,
            sso_token: None,
            status_message: None,
            profile_input: String::new(),
            profile_input_cursor: 0,
            pending_role: None,
            existing_profile_name: None,
            device_auth_info: None,
            device_auth_rx: None,
            last_ctrl_c_time: None,
            pending_confirm_action: None,
            sso_start_url_input: String::new(),
            sso_region_input: String::new(),
            sso_session_name_input: "default-sso".to_string(),
            sso_input_cursor: 0,
            editing_session_original_name: None,
            default_region_input: String::new(),
            default_output_input: String::new(),
            default_input_cursor: 0,
            new_profile_name_input: String::new(),
            new_profile_region_input: String::new(),
            new_profile_output_input: String::new(),
            new_profile_input_cursor: 0,
            static_profile_name_input: String::new(),
            static_access_key_input: String::new(),
            static_secret_key_input: String::new(),
            static_session_token_input: String::new(),
            static_input_cursor: 0,
            last_auto_refresh: None,
            theme: catppuccin::PALETTE.mocha,
            tick_count: 0,
            login_rx,
            login_tx,
            ssm_instances: Vec::new(),
            ssm_list_state: TableState::default(),
            ssm_filter: String::new(),
            ssm_search_mode: false,
            ssm_loading: false,
            ssm_show_offline: false,
            ssm_sort_order: SsmSortOrder::None,
        })
    }

    /// Get the currently selected SSO session
    fn get_selected_session(&self) -> Option<&SsoSessionInfo> {
        self.sessions_list_state
            .selected()
            .and_then(|idx| self.sso_sessions.get(idx))
    }

    /// Get the currently selected SSO session's token
    #[allow(dead_code)]
    fn get_current_token(&self) -> Option<&SsoToken> {
        self.get_selected_session()
            .and_then(|session| session.token.as_ref())
    }

    /// Get the currently selected SSO session's instance
    #[allow(dead_code)]
    fn get_current_instance(&self) -> Option<&SsoInstance> {
        self.get_selected_session().map(|session| &session.instance)
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode().map_err(SsoError::Io)?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(SsoError::Io)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(SsoError::Io)?;

        // Try to load cached profiles first for instant display
        if let Ok(Some(cached)) = cache::load_profiles() {
            tracing::debug!(
                "Loaded {} cached profiles ({})",
                cached.profiles.len(),
                cached.age_display()
            );
            self.load_profiles_from_cache(&cached);
            self.showing_cached_data = Some(cached);
        }

        // Load SSO sessions from disk (fast)
        self.load_all_sso_sessions().await;

        // Draw initial UI immediately so user sees something
        terminal.draw(|f| self.ui(f)).map_err(SsoError::Io)?;

        // Now load accounts from AWS API (can be slow)
        // Only if we have SSO sessions AND an active token
        if !self.sso_sessions.is_empty() && self.sso_token.is_some() {
            // Show loading message with spinner
            self.status_message = Some("Refreshing profile list...".to_string());
            terminal.draw(|f| self.ui(f)).map_err(SsoError::Io)?;

            let _ = self.load_accounts().await;

            // Clear status message after loading
            self.status_message = None;

            // If we successfully loaded accounts from an active session,
            // default to Accounts pane for better UX
            if !self.accounts.is_empty() {
                self.active_pane = ActivePane::Accounts;
                // Select first account
                self.accounts_list_state.select(Some(0));
            }
        }

        // If we still have no accounts (no SSO sessions or no token), load static profiles
        // This handles the case where user has only static credentials with no SSO configured
        if self.accounts.is_empty() {
            self.load_static_profiles_only();
            if !self.accounts.is_empty() {
                self.active_pane = ActivePane::Accounts;
                self.accounts_list_state.select(Some(0));
            }
        }

        // Main event loop
        let result = self.run_event_loop(&mut terminal).await;

        // Restore terminal
        disable_raw_mode().map_err(SsoError::Io)?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(SsoError::Io)?;
        terminal.show_cursor().map_err(SsoError::Io)?;

        result
    }

    async fn run_event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        // Refresh interval: 1 minute
        const AUTO_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);
        // Credential refresh threshold: 3 hours remaining (for 12h credentials, this is ~9h of age)
        const CREDENTIAL_REFRESH_THRESHOLD_MINUTES: i64 = 180;

        loop {
            terminal.draw(|f| self.ui(f)).map_err(SsoError::Io)?;

            // Increment tick counter for animations
            self.tick_count = self.tick_count.wrapping_add(1);

            // Check for login results from background tasks
            while let Ok(result) = self.login_rx.try_recv() {
                self.handle_login_result(result).await?;
            }

            // Check if we need to auto-refresh (every 1 minute)
            let now = std::time::Instant::now();
            let should_auto_refresh = match self.last_auto_refresh {
                Some(last_refresh) => now.duration_since(last_refresh) >= AUTO_REFRESH_INTERVAL,
                None => {
                    // First time - set the timer but don't refresh yet
                    self.last_auto_refresh = Some(now);
                    false
                }
            };

            if should_auto_refresh && self.state == AppState::Main && !self.accounts.is_empty() {
                self.last_auto_refresh = Some(now);

                // Always refresh SSO session statuses (checks token expiration)
                self.load_all_sso_sessions().await;

                if self.sso_token.is_some() {
                    // Full refresh with AWS API
                    tracing::debug!("Auto-refreshing account list (1 minute interval)");
                    if let Err(e) = self.load_accounts().await {
                        tracing::warn!("Auto-refresh failed: {}", e);
                    }

                    // Auto-refresh expiring credentials (proactive refresh before they expire)
                    self.auto_refresh_expiring_credentials(CREDENTIAL_REFRESH_THRESHOLD_MINUTES)
                        .await;
                } else {
                    // Light refresh: just update credential statuses from local files
                    tracing::debug!("Auto-refreshing credential statuses from local files");
                    self.refresh_credential_statuses();
                }
            }

            if event::poll(std::time::Duration::from_millis(250)).map_err(SsoError::Io)? {
                if let Event::Key(key) = event::read().map_err(SsoError::Io)? {
                    // Only handle key press events, ignore key release
                    if key.kind == KeyEventKind::Press {
                        // Check for Ctrl+C
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c')
                        {
                            self.handle_ctrl_c();
                        } else {
                            self.handle_key(key.code).await?;
                        }
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    /// Handle login result from background task
    async fn handle_login_result(&mut self, result: LoginResult) -> Result<()> {
        match result {
            LoginResult::Success {
                session_index,
                token,
                instance,
                session_name,
            } => {
                self.device_auth_info = None;
                self.device_auth_rx = None;

                // Update session in list (unbox the token)
                let token = *token;
                if let Some(session_mut) = self.sso_sessions.get_mut(session_index) {
                    session_mut.is_active = true;
                    session_mut.token = Some(token.clone());
                    session_mut.token_expiration = Some(token.expires_at);
                }

                // Update current session
                self.sso_instance = Some(instance);
                self.sso_token = Some(token);
                self.state = AppState::Main;
                self.status_message = Some(format!("✓ Logged in to {}", session_name));

                // Load accounts for this session
                self.load_accounts().await?;

                // Switch to Accounts pane for better UX
                if !self.accounts.is_empty() {
                    self.active_pane = ActivePane::Accounts;
                    self.accounts_list_state.select(Some(0));
                }
            }
            LoginResult::Error { message } => {
                self.device_auth_info = None;
                self.device_auth_rx = None;
                self.state = AppState::Main;
                self.status_message = Some(format!("Login failed: {}", message));
            }
            LoginResult::Cancelled => {
                self.device_auth_info = None;
                self.device_auth_rx = None;
                self.state = AppState::Main;
                self.status_message = Some("Login cancelled".to_string());
            }
        }
        Ok(())
    }

    async fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        match self.state {
            AppState::Main => self.handle_main_key(key).await?,
            AppState::Help => {
                // Any key exits help screen
                self.state = AppState::Main;
            }
            AppState::Loading => {
                // Allow cancelling login with q or Esc
                match key {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        // Cancel the login attempt
                        tracing::info!("User cancelled login");
                        self.device_auth_info = None;
                        self.device_auth_rx = None;
                        self.state = AppState::Main;
                        self.status_message = Some("Login cancelled".to_string());
                        // Note: The background task will still complete, but we ignore its result
                    }
                    _ => {}
                }
            }
            AppState::Error(_) => {
                // Any key clears error and returns to main
                self.state = AppState::Main;
            }
            AppState::ProfileInput => {
                self.handle_profile_input_key(key).await?;
            }
            AppState::SsoConfigInput { .. } => {
                self.handle_sso_config_input_key(key).await?;
            }
            AppState::DefaultsConfigInput { .. } => {
                self.handle_defaults_config_input_key(key).await?;
            }
            AppState::NewProfileConfigInput { .. } => {
                self.handle_new_profile_config_input_key(key).await?;
            }
            AppState::StaticCredentialInput { .. } => {
                self.handle_static_credential_input_key(key).await?;
            }
            AppState::ConfirmationDialog { .. } => {
                self.handle_confirmation_dialog_key(key).await?;
            }
            AppState::ViewProfile { .. } => {
                // Any key exits view profile screen
                self.state = AppState::Main;
            }
            AppState::SsmBrowser => {
                self.handle_ssm_browser_key(key).await?;
            }
            AppState::ViewInstanceTags { .. } => {
                // Any key returns to SSM browser
                self.state = AppState::SsmBrowser;
            }
        }
        Ok(())
    }

    fn handle_ctrl_c(&mut self) {
        let now = std::time::Instant::now();

        if let Some(last_press) = self.last_ctrl_c_time {
            // Check if within 2 seconds
            if now.duration_since(last_press).as_secs() < 2 {
                // Double press detected - force quit
                tracing::info!("Ctrl+C pressed twice - forcing exit");
                self.should_quit = true;
                return;
            }
        }

        // First press or too long since last press
        self.last_ctrl_c_time = Some(now);
        self.status_message = Some("Press Ctrl+C again within 2 seconds to force quit".to_string());
    }

    async fn handle_main_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.state = AppState::Help;
            }
            KeyCode::Tab => {
                // Switch between Sessions and Accounts panes
                self.active_pane = match self.active_pane {
                    ActivePane::Sessions => ActivePane::Accounts,
                    ActivePane::Accounts => ActivePane::Sessions,
                };
                self.status_message = Some(format!(
                    "Switched to {} pane",
                    match self.active_pane {
                        ActivePane::Sessions => "Sessions",
                        ActivePane::Accounts => "Accounts",
                    }
                ));
            }
            KeyCode::Char('r') => {
                // Reload SSO sessions from disk first (picks up external auth changes)
                let selected_idx = self.sessions_list_state.selected();
                self.load_all_sso_sessions().await;
                // Restore selection
                if let Some(idx) = selected_idx {
                    if idx < self.sso_sessions.len() {
                        self.sessions_list_state.select(Some(idx));
                        // Update sso_token from reloaded session
                        if let Some(session) = self.sso_sessions.get(idx) {
                            self.sso_token = session.token.clone();
                            self.sso_instance = Some(session.instance.clone());
                        }
                    }
                }

                // Refresh account list (bypass cache)
                if self.sso_token.is_some() {
                    // Clear cache for current session to force fresh fetch
                    if let Some(session_name) =
                        self.get_selected_session().map(|s| s.session_name.clone())
                    {
                        if self.accounts_cache.remove(&session_name).is_some() {
                            tracing::debug!("Cleared cache for session: {}", session_name);
                        }
                    }
                    self.load_accounts().await?;
                    // Reset auto-refresh timer after manual refresh
                    self.last_auto_refresh = Some(std::time::Instant::now());
                    self.status_message = Some("Refreshed sessions and accounts".to_string());
                } else {
                    // Still show that we refreshed sessions even if no token
                    self.status_message = Some(
                        "Refreshed sessions. No active token - press Enter on a session to login."
                            .to_string(),
                    );
                }
            }
            KeyCode::Down | KeyCode::Char('j') => match self.active_pane {
                ActivePane::Sessions => self.next_session(),
                ActivePane::Accounts => self.next_item(),
            },
            KeyCode::Up | KeyCode::Char('k') => match self.active_pane {
                ActivePane::Sessions => self.previous_session(),
                ActivePane::Accounts => self.previous_item(),
            },
            KeyCode::Enter => {
                match self.active_pane {
                    ActivePane::Sessions => {
                        // Start or stop SSO session
                        self.toggle_sso_session().await?;
                    }
                    ActivePane::Accounts => {
                        // Start or stop role session
                        self.toggle_role_session().await?;
                    }
                }
            }
            KeyCode::Char('a') => match self.active_pane {
                ActivePane::Sessions => {
                    self.add_sso_session().await?;
                }
                ActivePane::Accounts => {
                    self.add_static_credential().await?;
                }
            },
            KeyCode::Char('e') => {
                match self.active_pane {
                    ActivePane::Sessions => {
                        self.edit_sso_session().await?;
                    }
                    ActivePane::Accounts => {
                        // Edit profile (name, region, output)
                        self.edit_profile().await?;
                    }
                }
            }
            KeyCode::Char('d') => {
                match self.active_pane {
                    ActivePane::Sessions => {
                        self.delete_sso_session().await?;
                    }
                    ActivePane::Accounts => {
                        // Set as default profile
                        self.set_as_default().await?;
                    }
                }
            }
            KeyCode::Char('D') => {
                if self.active_pane == ActivePane::Accounts {
                    // Delete static credential (Shift+D)
                    self.delete_static_profile().await?;
                }
            }
            KeyCode::Char('c') => {
                if self.active_pane == ActivePane::Accounts {
                    // Open AWS Console in browser
                    self.open_console().await?;
                }
            }
            KeyCode::Char('f') => {
                if self.active_pane == ActivePane::Sessions {
                    // Toggle session filter
                    self.toggle_session_filter().await?;
                }
            }
            KeyCode::Char('v') => {
                if self.active_pane == ActivePane::Accounts {
                    // View profile details
                    self.view_profile_details();
                }
            }
            KeyCode::Char('s') => {
                if self.active_pane == ActivePane::Accounts {
                    // Open SSM browser
                    self.open_ssm_browser().await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn next_item(&mut self) {
        if self.accounts.is_empty() {
            return;
        }
        let i = match self.accounts_list_state.selected() {
            Some(i) => {
                if i >= self.accounts.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.accounts_list_state.select(Some(i));
    }

    fn previous_item(&mut self) {
        if self.accounts.is_empty() {
            return;
        }
        let i = match self.accounts_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.accounts.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.accounts_list_state.select(Some(i));
    }

    fn next_session(&mut self) {
        if self.sso_sessions.is_empty() {
            return;
        }
        let i = match self.sessions_list_state.selected() {
            Some(i) => {
                if i >= self.sso_sessions.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.sessions_list_state.select(Some(i));
        // Update current session
        self.update_current_session_from_selection();

        // Show which session is now selected
        if let Some(session) = self.sso_sessions.get(i) {
            if session.is_active {
                self.status_message = Some(format!(
                    "Selected session '{}' - press 'r' in Accounts pane to load accounts",
                    session.session_name
                ));
            } else {
                self.status_message = Some(format!(
                    "Selected session '{}' (inactive - press Enter to login)",
                    session.session_name
                ));
            }
        }
    }

    fn previous_session(&mut self) {
        if self.sso_sessions.is_empty() {
            return;
        }
        let i = match self.sessions_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.sso_sessions.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.sessions_list_state.select(Some(i));
        // Update current session
        self.update_current_session_from_selection();

        // Show which session is now selected
        if let Some(session) = self.sso_sessions.get(i) {
            if session.is_active {
                self.status_message = Some(format!(
                    "Selected session '{}' - press 'r' in Accounts pane to load accounts",
                    session.session_name
                ));
            } else {
                self.status_message = Some(format!(
                    "Selected session '{}' (inactive - press Enter to login)",
                    session.session_name
                ));
            }
        }
    }

    /// Update current sso_instance and sso_token based on selected session
    fn update_current_session_from_selection(&mut self) {
        let selected_idx = self.sessions_list_state.selected();
        if let Some(idx) = selected_idx {
            if let Some(session) = self.sso_sessions.get(idx) {
                self.sso_instance = Some(session.instance.clone());
                self.sso_token = session.token.clone();
                return;
            }
        }
        self.sso_instance = None;
        self.sso_token = None;
    }

    /// Toggle SSO session: if active, logout; if inactive, login
    async fn toggle_sso_session(&mut self) -> Result<()> {
        if let Some(index) = self.sessions_list_state.selected() {
            if let Some(session) = self.sso_sessions.get(index).cloned() {
                if session.is_active {
                    // Session is active, logout
                    self.logout_session(index).await?;
                } else {
                    // Session is inactive, login
                    self.login_session(index).await?;
                }
            }
        } else {
            self.status_message = Some("No session selected".to_string());
        }
        Ok(())
    }

    /// Login to a specific SSO session by index
    async fn login_session(&mut self, index: usize) -> Result<()> {
        if let Some(session) = self.sso_sessions.get(index).cloned() {
            self.status_message = Some(format!("Logging in to {}...", session.session_name));
            self.state = AppState::Loading;

            let instance = session.instance.clone();
            let session_name = session.session_name.clone();
            let tx = self.login_tx.clone();

            // Create watch channel for sharing device auth info with background task
            let (device_auth_tx, device_auth_rx) =
                tokio::sync::watch::channel::<Option<DeviceAuthorizationInfo>>(None);

            // Spawn background task for login
            tokio::spawn(async move {
                // Create new AuthManager for this task
                let auth_manager = match AuthManager::new() {
                    Ok(am) => am,
                    Err(e) => {
                        let _ = tx.send(LoginResult::Error {
                            message: format!("Failed to create auth manager: {}", e),
                        });
                        return;
                    }
                };

                // Perform login with callback
                let result = auth_manager
                    .login_with_callback(&instance, false, |auth_info| {
                        // Send auth info for TUI to display via watch channel
                        let _ = device_auth_tx.send(Some(auth_info.clone()));

                        // Only try to open browser if not in headless environment
                        if !crate::env::is_headless_environment() {
                            let url_to_open = auth_info
                                .verification_uri_complete
                                .as_ref()
                                .unwrap_or(&auth_info.verification_uri);

                            if let Err(e) = webbrowser::open(url_to_open) {
                                tracing::warn!("Could not open browser automatically: {}", e);
                            }
                        } else {
                            tracing::info!("Headless environment detected - skipping browser launch, showing URL in TUI");
                        }

                        Ok(())
                    })
                    .await;

                // Send result back to main thread
                let message = match result {
                    Ok(token) => LoginResult::Success {
                        session_index: index,
                        token: Box::new(token),
                        instance,
                        session_name,
                    },
                    Err(e) => LoginResult::Error {
                        message: format!("{}", e),
                    },
                };

                let _ = tx.send(message);
            });

            // Store the watch receiver so we can poll it during rendering
            self.device_auth_rx = Some(device_auth_rx);
        }
        Ok(())
    }

    /// Logout from a specific SSO session by index
    async fn logout_session(&mut self, index: usize) -> Result<()> {
        if let Some(session) = self.sso_sessions.get_mut(index) {
            self.status_message = Some(format!("Logging out from {}...", session.session_name));

            // Remove cached token
            if let Err(e) = self.auth_manager.remove_token(&session.instance) {
                tracing::warn!("Failed to remove cached token: {}", e);
            }

            // Update session status
            session.is_active = false;
            session.token = None;
            session.token_expiration = None;

            // If this was the current session, clear token but keep profiles visible
            // (credentials may still be valid even without active SSO session)
            if let Some(ref current_instance) = self.sso_instance {
                if current_instance.start_url == session.start_url {
                    self.sso_instance = None;
                    self.sso_token = None;
                    // Don't clear accounts - profiles remain visible with their credential status
                }
            }

            self.status_message = Some(format!("✓ Logged out from {}", session.session_name));
        }
        Ok(())
    }

    /// Add a new SSO session
    async fn add_sso_session(&mut self) -> Result<()> {
        // Clear input buffers for fresh start
        self.sso_start_url_input.clear();
        self.sso_region_input.clear();
        self.sso_session_name_input = "default-sso".to_string();
        self.sso_input_cursor = 0;
        // Clear original name to indicate new session (not editing)
        self.editing_session_original_name = None;

        // Show SSO configuration input dialog
        self.state = AppState::SsoConfigInput {
            step: SsoConfigStep::StartUrl,
        };
        self.status_message = Some("Add new SSO session".to_string());
        Ok(())
    }

    /// Add a new static credential profile
    async fn add_static_credential(&mut self) -> Result<()> {
        // Clear input buffers for fresh start
        self.static_profile_name_input.clear();
        self.static_access_key_input.clear();
        self.static_secret_key_input.clear();
        self.static_session_token_input.clear();
        self.static_input_cursor = 0;

        // Show static credential input dialog
        self.state = AppState::StaticCredentialInput {
            step: StaticCredentialStep::ProfileName,
        };
        self.status_message = Some("Add new static credential profile".to_string());
        Ok(())
    }

    /// Delete the selected static credential profile
    async fn delete_static_profile(&mut self) -> Result<()> {
        if let Some(index) = self.accounts_list_state.selected() {
            if let Some(profile_entry) = self.accounts.get(index).cloned() {
                match profile_entry {
                    ProfileEntry::Static { profile_name, .. } => {
                        // Delete the static credential
                        match crate::aws_config::delete_static_credentials(&profile_name) {
                            Ok(()) => {
                                self.status_message = Some(format!(
                                    "✓ Deleted static credential profile '{}'",
                                    profile_name
                                ));

                                // Reload accounts to update the list
                                if let Err(e) = self.load_accounts().await {
                                    tracing::warn!(
                                        "Failed to reload accounts after deletion: {}",
                                        e
                                    );
                                }
                            }
                            Err(e) => {
                                self.status_message =
                                    Some(format!("Error deleting profile: {}", e));
                            }
                        }
                    }
                    ProfileEntry::Sso(_) => {
                        self.status_message = Some("Use lowercase 'd' to set as default. SSO profiles cannot be deleted from here.".to_string());
                    }
                    ProfileEntry::Incomplete { profile_name, .. } => {
                        self.status_message = Some(format!(
                            "Profile '{}' has no credentials to delete. It only exists in config.",
                            profile_name
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Edit the selected SSO session
    async fn edit_sso_session(&mut self) -> Result<()> {
        if let Some(index) = self.sessions_list_state.selected() {
            if let Some(session) = self.sso_sessions.get(index) {
                // Pre-fill input buffers with current session values
                self.sso_start_url_input = session.start_url.clone();
                self.sso_region_input = session.region.clone();
                self.sso_session_name_input = session.session_name.clone();
                self.sso_input_cursor = self.sso_start_url_input.len();
                // Track original name for rename detection
                self.editing_session_original_name = Some(session.session_name.clone());

                // Show SSO configuration input dialog
                self.state = AppState::SsoConfigInput {
                    step: SsoConfigStep::StartUrl,
                };
                self.status_message = Some(format!("Edit SSO session '{}'", session.session_name));
            }
        } else {
            self.status_message = Some("No session selected".to_string());
        }
        Ok(())
    }

    /// Delete the selected SSO session (requires confirmation via modal dialog)
    async fn delete_sso_session(&mut self) -> Result<()> {
        if let Some(index) = self.sessions_list_state.selected() {
            if let Some(session) = self.sso_sessions.get(index) {
                let message = vec![
                    format!(
                        "Are you sure you want to delete SSO session '{}'?",
                        session.session_name
                    ),
                    "".to_string(),
                    format!("Start URL: {}", session.start_url),
                    format!("Region: {}", session.region),
                    "".to_string(),
                    "This will remove the session from ~/.aws/config.".to_string(),
                    if session.is_active {
                        "The session will be logged out first.".to_string()
                    } else {
                        "".to_string()
                    },
                ];

                // Show confirmation dialog
                self.pending_confirm_action = Some(ConfirmAction::DeleteSession {
                    session_index: index,
                    session_name: session.session_name.clone(),
                });
                self.state = AppState::ConfirmationDialog {
                    title: "Delete SSO Session".to_string(),
                    message,
                };
            }
        } else {
            self.status_message = Some("No session selected".to_string());
        }
        Ok(())
    }

    /// Toggle session filter: filter accounts by selected session
    async fn toggle_session_filter(&mut self) -> Result<()> {
        if let Some(index) = self.sessions_list_state.selected() {
            if let Some(session) = self.sso_sessions.get(index) {
                let session_name = session.session_name.clone();

                // Toggle filter
                if self.filtered_session == Some(session_name.clone()) {
                    // Already filtered on this session - unfilter
                    self.filtered_session = None;
                    self.status_message =
                        Some(format!("Filter removed for session '{}'", session_name));
                } else {
                    // Filter on this session
                    self.filtered_session = Some(session_name.clone());
                    self.status_message = Some(format!(
                        "Filtering accounts by session '{}' (press 'f' again to unfilter)",
                        session_name
                    ));
                }

                // Reload accounts to apply filter
                self.load_accounts().await?;
            }
        } else {
            self.status_message = Some("No session selected".to_string());
        }
        Ok(())
    }

    /// Toggle role session: if active, delete it; if inactive, create it
    async fn toggle_role_session(&mut self) -> Result<()> {
        if let Some(index) = self.accounts_list_state.selected() {
            if let Some(profile_entry) = self.accounts.get(index).cloned() {
                // Only SSO profiles can be toggled (static credentials don't have sessions)
                let account_with_status = match profile_entry {
                    ProfileEntry::Sso(status) => status,
                    ProfileEntry::Static { .. } => {
                        self.status_message = Some("Static credentials cannot be toggled. Use 'e' to edit or 'd' to delete.".to_string());
                        return Ok(());
                    }
                    ProfileEntry::Incomplete { profile_name, .. } => {
                        self.status_message = Some(format!(
                            "Profile '{}' has no credentials. Add credentials first.",
                            profile_name
                        ));
                        return Ok(());
                    }
                };

                let account = account_with_status.account_role;

                if account_with_status.is_active {
                    // Role is active, stop it (delete credentials)
                    if let Some(existing_profile) =
                        crate::aws_config::get_existing_profile_name(&account)?
                    {
                        self.status_message = Some(format!(
                            "Stopping session for profile '{}'...",
                            existing_profile
                        ));
                        if let Err(e) = crate::aws_config::invalidate_profile(&existing_profile) {
                            self.status_message = Some(format!("Error stopping session: {}", e));
                        } else {
                            self.status_message = Some(format!(
                                "✓ Stopped session for profile '{}' (profile preserved)",
                                existing_profile
                            ));
                            // Reload accounts to update indicators
                            if let Err(e) = self.load_accounts().await {
                                tracing::warn!(
                                    "Failed to reload accounts after stopping session: {}",
                                    e
                                );
                            }
                        }
                    }
                } else {
                    // Role is inactive, start it (get credentials)
                    // Get current session name for unified profile lookup
                    let session_name = if let Some(selected_session) = self.get_selected_session() {
                        selected_session.session_name.clone()
                    } else {
                        self.status_message = Some("No SSO session selected".to_string());
                        return Ok(());
                    };

                    // Check if there's an existing profile for this role using unified lookup
                    let existing_profile = crate::aws_config::get_profile_by_role(
                        &session_name,
                        &account.account_id,
                        &account.role_name,
                    )?;

                    if let Some(profile_info) = existing_profile {
                        // Profile exists, just activate it (fetch credentials only)
                        // Set existing_profile_name so save_profile_credentials doesn't
                        // incorrectly treat this as overwriting a different profile
                        self.existing_profile_name = Some(profile_info.name.clone());
                        self.state = AppState::Loading;
                        self.save_profile_credentials(&account, &profile_info.name)
                            .await?;
                    } else {
                        // First time creating profile for this role
                        // Check if awsom defaults exist
                        match crate::aws_config::read_awsom_defaults()? {
                            Some(defaults) => {
                                // Defaults exist, show new profile config dialog
                                let default_profile_name = format!(
                                    "{}_{}",
                                    account
                                        .account_name
                                        .replace(" ", "-")
                                        .replace("_", "-")
                                        .to_lowercase(),
                                    account
                                        .role_name
                                        .replace(" ", "-")
                                        .replace("_", "-")
                                        .to_lowercase()
                                );
                                self.new_profile_name_input = default_profile_name;
                                self.new_profile_region_input = defaults.region.clone();
                                self.new_profile_output_input = defaults.output.clone();
                                self.new_profile_input_cursor = self.new_profile_name_input.len();
                                self.pending_role = Some(account);
                                self.state = AppState::NewProfileConfigInput {
                                    step: NewProfileConfigStep::ProfileName,
                                };
                                self.status_message =
                                    Some("Configure profile for this role".to_string());
                            }
                            None => {
                                // No awsom defaults found, show defaults config dialog first
                                self.pending_role = Some(account);
                                self.state = AppState::DefaultsConfigInput {
                                    step: DefaultsConfigStep::Region,
                                };
                                self.status_message = Some(
                                    "Let's configure default settings for new profiles!"
                                        .to_string(),
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Set the selected role's profile as the default profile
    async fn set_as_default(&mut self) -> Result<()> {
        if let Some(index) = self.accounts_list_state.selected() {
            if let Some(profile_entry) = self.accounts.get(index).cloned() {
                // Get the existing profile name and account (if SSO) based on entry type
                let (existing_profile, account_opt) = match profile_entry {
                    ProfileEntry::Sso(ref account_with_status) => {
                        let account = &account_with_status.account_role;
                        let prof = crate::aws_config::get_existing_profile_name(account)?;
                        (prof, Some(account.clone()))
                    }
                    ProfileEntry::Static {
                        ref profile_name, ..
                    } => (Some(profile_name.clone()), None),
                    ProfileEntry::Incomplete {
                        ref profile_name, ..
                    } => {
                        self.status_message = Some(format!(
                            "Profile '{}' has no credentials. Cannot set as default.",
                            profile_name
                        ));
                        return Ok(());
                    }
                };

                // Check if there's an existing profile
                if let Some(existing_profile) = existing_profile {
                    // Don't rename if already default
                    if existing_profile == "default" {
                        self.status_message = Some("Profile is already set as default".to_string());
                        return Ok(());
                    }

                    // Check if [default] profile already exists
                    match crate::aws_config::get_profile_details("default") {
                        Ok(Some(details)) => {
                            // Default profile exists - show confirmation dialog
                            let mut message = vec![
                                "Profile [default] already exists.".to_string(),
                                "".to_string(),
                            ];

                            // Combine region and output on one line if both exist
                            let mut settings = Vec::new();
                            if let Some(region) = details.region {
                                settings.push(format!("region={}", region));
                            }
                            if let Some(output) = details.output {
                                settings.push(format!("output={}", output));
                            }
                            if !settings.is_empty() {
                                message.push(format!("Current: {}", settings.join(", ")));
                            }

                            // Show SSO details if present (compact)
                            if details.sso_session.is_some()
                                || details.sso_account_id.is_some()
                                || details.sso_role_name.is_some()
                            {
                                let mut sso_parts = Vec::new();
                                if let Some(session) = details.sso_session {
                                    sso_parts.push(format!("session={}", session));
                                }
                                if let Some(account) = details.sso_account_id {
                                    sso_parts.push(format!("account={}", account));
                                }
                                if let Some(role) = details.sso_role_name {
                                    sso_parts.push(format!("role={}", role));
                                }
                                message.push(format!("SSO: {}", sso_parts.join(", ")));
                            }
                            message.push("".to_string());
                            message.push(format!("Replace with '{}'?", existing_profile));

                            // Show confirmation dialog
                            let account = account_opt.unwrap_or(AccountRole {
                                account_id: String::new(),
                                account_name: String::new(),
                                role_name: String::new(),
                            });
                            self.pending_confirm_action = Some(ConfirmAction::MakeProfileDefault {
                                from_profile: existing_profile,
                                account,
                            });
                            self.state = AppState::ConfirmationDialog {
                                title: "Replace [default] Profile".to_string(),
                                message,
                            };
                        }
                        Ok(None) => {
                            // No default profile - proceed directly
                            match crate::aws_config::rename_profile(&existing_profile, "default") {
                                Ok(()) => {
                                    self.status_message = Some(format!(
                                        "✓ Set '{}' as default profile",
                                        existing_profile
                                    ));
                                    // Reload accounts to update indicators
                                    if let Err(e) = self.load_accounts().await {
                                        tracing::warn!(
                                            "Failed to reload accounts after setting default: {}",
                                            e
                                        );
                                    }
                                }
                                Err(e) => {
                                    self.status_message =
                                        Some(format!("Error setting default profile: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            self.status_message =
                                Some(format!("Error checking default profile: {}", e));
                        }
                    }
                } else {
                    self.status_message = Some("No active profile found for this role. Press Enter to create credentials first.".to_string());
                }
            }
        }
        Ok(())
    }

    /// Open profile editor for selected role (name, region, output)
    async fn edit_profile(&mut self) -> Result<()> {
        if let Some(index) = self.accounts_list_state.selected() {
            if let Some(profile_entry) = self.accounts.get(index).cloned() {
                // Handle SSO vs Static differently
                match profile_entry {
                    ProfileEntry::Static { .. } => {
                        self.status_message =
                            Some("Static credential editing coming soon (Step 3.6)".to_string());
                        return Ok(());
                    }
                    ProfileEntry::Sso(account_with_status) => {
                        let account = account_with_status.account_role;

                        // Get current session name for unified profile lookup
                        let session_name =
                            if let Some(selected_session) = self.get_selected_session() {
                                selected_session.session_name.clone()
                            } else {
                                self.status_message = Some("No SSO session selected".to_string());
                                return Ok(());
                            };

                        // Look up existing profile using unified lookup
                        let existing_profile = crate::aws_config::get_profile_by_role(
                            &session_name,
                            &account.account_id,
                            &account.role_name,
                        )?;

                        if let Some(profile_info) = existing_profile {
                            // Edit existing profile - pre-fill with current values
                            self.new_profile_name_input = profile_info.name.clone();
                            self.new_profile_region_input = profile_info.region;
                            self.new_profile_output_input = profile_info.output;
                            self.new_profile_input_cursor = self.new_profile_name_input.len();
                            self.existing_profile_name = Some(profile_info.name);
                        } else {
                            // Create new profile - use defaults
                            let default_profile_name = format!(
                                "{}_{}",
                                account
                                    .account_name
                                    .replace(" ", "-")
                                    .replace("_", "-")
                                    .to_lowercase(),
                                account
                                    .role_name
                                    .replace(" ", "-")
                                    .replace("_", "-")
                                    .to_lowercase()
                            );
                            self.new_profile_name_input = default_profile_name;

                            // Try to get defaults from awsom-defaults
                            match crate::aws_config::read_awsom_defaults()? {
                                Some(defaults) => {
                                    self.new_profile_region_input = defaults.region;
                                    self.new_profile_output_input = defaults.output;
                                }
                                None => {
                                    // Use hardcoded fallback if awsom-defaults doesn't exist
                                    self.new_profile_region_input = "us-east-1".to_string();
                                    self.new_profile_output_input = "json".to_string();
                                }
                            }

                            self.new_profile_input_cursor = self.new_profile_name_input.len();
                            self.existing_profile_name = None;
                        }

                        self.pending_role = Some(account);
                        self.state = AppState::NewProfileConfigInput {
                            step: NewProfileConfigStep::ProfileName,
                        };
                        self.status_message = Some("Edit profile configuration".to_string());
                    }
                    ProfileEntry::Incomplete { profile_name, .. } => {
                        self.status_message = Some(format!(
                            "Profile '{}' has no credentials. Add credentials first.",
                            profile_name
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_profile_input_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Enter => {
                // Save profile with entered name
                if let Some(account) = self.pending_role.take() {
                    self.state = AppState::Loading;
                    self.save_profile_credentials(&account, &self.profile_input.clone())
                        .await?;
                }
            }
            KeyCode::Esc => {
                // Cancel
                self.state = AppState::Main;
                self.profile_input.clear();
                self.profile_input_cursor = 0;
                self.pending_role = None;
                self.existing_profile_name = None;
            }
            KeyCode::Left => {
                // Move cursor left
                if self.profile_input_cursor > 0 {
                    self.profile_input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                // Move cursor right
                if self.profile_input_cursor < self.profile_input.len() {
                    self.profile_input_cursor += 1;
                }
            }
            KeyCode::Home => {
                // Move cursor to beginning
                self.profile_input_cursor = 0;
            }
            KeyCode::End => {
                // Move cursor to end
                self.profile_input_cursor = self.profile_input.len();
            }
            KeyCode::Backspace => {
                // Delete character before cursor
                if self.profile_input_cursor > 0 {
                    self.profile_input.remove(self.profile_input_cursor - 1);
                    self.profile_input_cursor -= 1;
                }
            }
            KeyCode::Delete => {
                // Delete character at cursor
                if self.profile_input_cursor < self.profile_input.len() {
                    self.profile_input.remove(self.profile_input_cursor);
                }
            }
            KeyCode::Char(c) => {
                // Only allow alphanumeric, dash, and underscore
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    self.profile_input.insert(self.profile_input_cursor, c);
                    self.profile_input_cursor += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_sso_config_input_key(&mut self, key: KeyCode) -> Result<()> {
        let current_step = if let AppState::SsoConfigInput { step } = &self.state {
            step.clone()
        } else {
            return Ok(());
        };

        match key {
            KeyCode::Enter => {
                // Move to next step or save configuration
                match current_step {
                    SsoConfigStep::StartUrl => {
                        if self.sso_start_url_input.trim().is_empty() {
                            self.status_message = Some("SSO Start URL is required".to_string());
                        } else {
                            self.state = AppState::SsoConfigInput {
                                step: SsoConfigStep::Region,
                            };
                            self.sso_input_cursor = self.sso_region_input.len();
                        }
                    }
                    SsoConfigStep::Region => {
                        if self.sso_region_input.trim().is_empty() {
                            self.status_message = Some("SSO Region is required".to_string());
                        } else {
                            self.state = AppState::SsoConfigInput {
                                step: SsoConfigStep::SessionName,
                            };
                            self.sso_input_cursor = self.sso_session_name_input.len();
                        }
                    }
                    SsoConfigStep::SessionName => {
                        // Save configuration to ~/.aws/config
                        let session_name = if self.sso_session_name_input.trim().is_empty() {
                            "default-sso".to_string()
                        } else {
                            self.sso_session_name_input.trim().to_string()
                        };

                        let session = crate::aws_config::SsoSession {
                            session_name: session_name.clone(),
                            sso_start_url: self.sso_start_url_input.trim().to_string(),
                            sso_region: self.sso_region_input.trim().to_string(),
                            sso_registration_scopes: "sso:account:access".to_string(),
                        };

                        // Pass old name for rename detection (handles profile updates)
                        match crate::aws_config::write_sso_session(
                            &session,
                            self.editing_session_original_name.as_deref(),
                        ) {
                            Ok(()) => {
                                self.status_message = Some(format!(
                                    "✓ SSO session '{}' saved to ~/.aws/config",
                                    session_name
                                ));
                                self.state = AppState::Main;

                                // Clear input buffers
                                self.sso_start_url_input.clear();
                                self.sso_region_input.clear();
                                self.sso_session_name_input = "default-sso".to_string();
                                self.sso_input_cursor = 0;
                                self.editing_session_original_name = None;

                                // Reload sessions list to show the new session
                                self.load_all_sso_sessions().await;
                            }
                            Err(e) => {
                                self.status_message =
                                    Some(format!("Error saving configuration: {}", e));
                            }
                        }
                    }
                }
            }
            KeyCode::Esc => {
                // Cancel configuration
                self.state = AppState::Main;
                self.sso_start_url_input.clear();
                self.sso_region_input.clear();
                self.sso_session_name_input = "default-sso".to_string();
                self.sso_input_cursor = 0;
                self.editing_session_original_name = None;
                self.status_message = Some("Configuration cancelled".to_string());
            }
            KeyCode::Left => {
                if self.sso_input_cursor > 0 {
                    self.sso_input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let max_len = match current_step {
                    SsoConfigStep::StartUrl => self.sso_start_url_input.len(),
                    SsoConfigStep::Region => self.sso_region_input.len(),
                    SsoConfigStep::SessionName => self.sso_session_name_input.len(),
                };
                if self.sso_input_cursor < max_len {
                    self.sso_input_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.sso_input_cursor = 0;
            }
            KeyCode::End => {
                self.sso_input_cursor = match current_step {
                    SsoConfigStep::StartUrl => self.sso_start_url_input.len(),
                    SsoConfigStep::Region => self.sso_region_input.len(),
                    SsoConfigStep::SessionName => self.sso_session_name_input.len(),
                };
            }
            KeyCode::Backspace => {
                if self.sso_input_cursor > 0 {
                    match current_step {
                        SsoConfigStep::StartUrl => {
                            self.sso_start_url_input.remove(self.sso_input_cursor - 1);
                        }
                        SsoConfigStep::Region => {
                            self.sso_region_input.remove(self.sso_input_cursor - 1);
                        }
                        SsoConfigStep::SessionName => {
                            self.sso_session_name_input
                                .remove(self.sso_input_cursor - 1);
                        }
                    }
                    self.sso_input_cursor -= 1;
                }
            }
            KeyCode::Delete => match current_step {
                SsoConfigStep::StartUrl => {
                    if self.sso_input_cursor < self.sso_start_url_input.len() {
                        self.sso_start_url_input.remove(self.sso_input_cursor);
                    }
                }
                SsoConfigStep::Region => {
                    if self.sso_input_cursor < self.sso_region_input.len() {
                        self.sso_region_input.remove(self.sso_input_cursor);
                    }
                }
                SsoConfigStep::SessionName => {
                    if self.sso_input_cursor < self.sso_session_name_input.len() {
                        self.sso_session_name_input.remove(self.sso_input_cursor);
                    }
                }
            },
            KeyCode::Char(c) => {
                // Allow reasonable characters for URLs and region names
                match current_step {
                    SsoConfigStep::StartUrl => {
                        self.sso_start_url_input.insert(self.sso_input_cursor, c);
                        self.sso_input_cursor += 1;
                    }
                    SsoConfigStep::Region => {
                        // Only allow lowercase letters, digits, and hyphens for region
                        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
                            self.sso_region_input.insert(self.sso_input_cursor, c);
                            self.sso_input_cursor += 1;
                        }
                    }
                    SsoConfigStep::SessionName => {
                        // Allow alphanumeric, dash, and underscore
                        if c.is_alphanumeric() || c == '-' || c == '_' {
                            self.sso_session_name_input.insert(self.sso_input_cursor, c);
                            self.sso_input_cursor += 1;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_static_credential_input_key(&mut self, key: KeyCode) -> Result<()> {
        let current_step = if let AppState::StaticCredentialInput { step } = &self.state {
            step.clone()
        } else {
            return Ok(());
        };

        match key {
            KeyCode::Enter => {
                // Move to next step or save configuration
                match current_step {
                    StaticCredentialStep::ProfileName => {
                        if self.static_profile_name_input.trim().is_empty() {
                            self.status_message = Some("Profile name is required".to_string());
                        } else {
                            self.state = AppState::StaticCredentialInput {
                                step: StaticCredentialStep::AccessKeyId,
                            };
                            self.static_input_cursor = self.static_access_key_input.len();
                        }
                    }
                    StaticCredentialStep::AccessKeyId => {
                        if self.static_access_key_input.trim().is_empty() {
                            self.status_message = Some("Access Key ID is required".to_string());
                        } else {
                            self.state = AppState::StaticCredentialInput {
                                step: StaticCredentialStep::SecretAccessKey,
                            };
                            self.static_input_cursor = self.static_secret_key_input.len();
                        }
                    }
                    StaticCredentialStep::SecretAccessKey => {
                        if self.static_secret_key_input.trim().is_empty() {
                            self.status_message = Some("Secret Access Key is required".to_string());
                        } else {
                            self.state = AppState::StaticCredentialInput {
                                step: StaticCredentialStep::SessionToken,
                            };
                            self.static_input_cursor = self.static_session_token_input.len();
                        }
                    }
                    StaticCredentialStep::SessionToken => {
                        // Save static credentials (session token is optional)
                        let session_token = if self.static_session_token_input.trim().is_empty() {
                            None
                        } else {
                            Some(self.static_session_token_input.trim().to_string())
                        };

                        let creds = crate::models::StaticCredentials {
                            access_key_id: self.static_access_key_input.trim().to_string(),
                            secret_access_key: self.static_secret_key_input.trim().to_string(),
                            session_token,
                        };

                        // Validate credentials
                        if let Err(e) = creds.validate() {
                            self.status_message = Some(format!("Validation error: {}", e));
                            return Ok(());
                        }

                        let profile_name = self.static_profile_name_input.trim();
                        match crate::aws_config::write_static_credentials(profile_name, &creds) {
                            Ok(()) => {
                                self.status_message = Some(format!(
                                    "✓ Static credentials '{}' saved to ~/.aws/credentials",
                                    profile_name
                                ));
                                self.state = AppState::Main;

                                // Clear input buffers
                                self.static_profile_name_input.clear();
                                self.static_access_key_input.clear();
                                self.static_secret_key_input.clear();
                                self.static_session_token_input.clear();
                                self.static_input_cursor = 0;

                                // Reload accounts to show the new profile
                                if let Err(e) = self.load_accounts().await {
                                    tracing::warn!("Failed to reload accounts: {}", e);
                                }
                            }
                            Err(e) => {
                                self.status_message =
                                    Some(format!("Error saving credentials: {}", e));
                            }
                        }
                    }
                }
            }
            KeyCode::Esc => {
                // Cancel configuration
                self.state = AppState::Main;
                self.static_profile_name_input.clear();
                self.static_access_key_input.clear();
                self.static_secret_key_input.clear();
                self.static_session_token_input.clear();
                self.static_input_cursor = 0;
                self.status_message = Some("Configuration cancelled".to_string());
            }
            KeyCode::Left => {
                if self.static_input_cursor > 0 {
                    self.static_input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let max_len = match current_step {
                    StaticCredentialStep::ProfileName => self.static_profile_name_input.len(),
                    StaticCredentialStep::AccessKeyId => self.static_access_key_input.len(),
                    StaticCredentialStep::SecretAccessKey => self.static_secret_key_input.len(),
                    StaticCredentialStep::SessionToken => self.static_session_token_input.len(),
                };
                if self.static_input_cursor < max_len {
                    self.static_input_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.static_input_cursor = 0;
            }
            KeyCode::End => {
                self.static_input_cursor = match current_step {
                    StaticCredentialStep::ProfileName => self.static_profile_name_input.len(),
                    StaticCredentialStep::AccessKeyId => self.static_access_key_input.len(),
                    StaticCredentialStep::SecretAccessKey => self.static_secret_key_input.len(),
                    StaticCredentialStep::SessionToken => self.static_session_token_input.len(),
                };
            }
            KeyCode::Backspace => {
                if self.static_input_cursor > 0 {
                    match current_step {
                        StaticCredentialStep::ProfileName => {
                            self.static_profile_name_input
                                .remove(self.static_input_cursor - 1);
                        }
                        StaticCredentialStep::AccessKeyId => {
                            self.static_access_key_input
                                .remove(self.static_input_cursor - 1);
                        }
                        StaticCredentialStep::SecretAccessKey => {
                            self.static_secret_key_input
                                .remove(self.static_input_cursor - 1);
                        }
                        StaticCredentialStep::SessionToken => {
                            self.static_session_token_input
                                .remove(self.static_input_cursor - 1);
                        }
                    }
                    self.static_input_cursor -= 1;
                }
            }
            KeyCode::Delete => match current_step {
                StaticCredentialStep::ProfileName => {
                    if self.static_input_cursor < self.static_profile_name_input.len() {
                        self.static_profile_name_input
                            .remove(self.static_input_cursor);
                    }
                }
                StaticCredentialStep::AccessKeyId => {
                    if self.static_input_cursor < self.static_access_key_input.len() {
                        self.static_access_key_input
                            .remove(self.static_input_cursor);
                    }
                }
                StaticCredentialStep::SecretAccessKey => {
                    if self.static_input_cursor < self.static_secret_key_input.len() {
                        self.static_secret_key_input
                            .remove(self.static_input_cursor);
                    }
                }
                StaticCredentialStep::SessionToken => {
                    if self.static_input_cursor < self.static_session_token_input.len() {
                        self.static_session_token_input
                            .remove(self.static_input_cursor);
                    }
                }
            },
            KeyCode::Char(c) => {
                match current_step {
                    StaticCredentialStep::ProfileName => {
                        // Allow alphanumeric, dash, and underscore
                        if c.is_alphanumeric() || c == '-' || c == '_' {
                            self.static_profile_name_input
                                .insert(self.static_input_cursor, c);
                            self.static_input_cursor += 1;
                        }
                    }
                    StaticCredentialStep::AccessKeyId
                    | StaticCredentialStep::SecretAccessKey
                    | StaticCredentialStep::SessionToken => {
                        // Allow all printable ASCII characters except whitespace
                        if !c.is_whitespace() && c.is_ascii() {
                            let target_input = match current_step {
                                StaticCredentialStep::AccessKeyId => {
                                    &mut self.static_access_key_input
                                }
                                StaticCredentialStep::SecretAccessKey => {
                                    &mut self.static_secret_key_input
                                }
                                StaticCredentialStep::SessionToken => {
                                    &mut self.static_session_token_input
                                }
                                _ => unreachable!(),
                            };
                            target_input.insert(self.static_input_cursor, c);
                            self.static_input_cursor += 1;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_defaults_config_input_key(&mut self, key: KeyCode) -> Result<()> {
        let current_step = if let AppState::DefaultsConfigInput { step } = &self.state {
            step.clone()
        } else {
            return Ok(());
        };

        match key {
            KeyCode::Enter => {
                match current_step {
                    DefaultsConfigStep::Region => {
                        if self.default_region_input.trim().is_empty() {
                            self.status_message = Some("Region is required".to_string());
                        } else {
                            self.state = AppState::DefaultsConfigInput {
                                step: DefaultsConfigStep::Output,
                            };
                            self.default_input_cursor = self.default_output_input.len();
                        }
                    }
                    DefaultsConfigStep::Output => {
                        // Save default configuration to [profile awsom-defaults]
                        let config = crate::aws_config::DefaultConfig {
                            region: self.default_region_input.trim().to_string(),
                            output: self.default_output_input.trim().to_string(),
                        };

                        match crate::aws_config::write_awsom_defaults(&config) {
                            Ok(()) => {
                                self.status_message = Some(
                                    "✓ Default settings saved to [profile awsom-defaults]"
                                        .to_string(),
                                );

                                // Now proceed to new profile configuration
                                if let Some(account) = &self.pending_role {
                                    let default_profile_name = format!(
                                        "{}_{}",
                                        account
                                            .account_name
                                            .replace(" ", "-")
                                            .replace("_", "-")
                                            .to_lowercase(),
                                        account
                                            .role_name
                                            .replace(" ", "-")
                                            .replace("_", "-")
                                            .to_lowercase()
                                    );
                                    self.new_profile_name_input = default_profile_name;
                                    self.new_profile_region_input = config.region.clone();
                                    self.new_profile_output_input = config.output.clone();
                                    self.new_profile_input_cursor =
                                        self.new_profile_name_input.len();
                                    self.state = AppState::NewProfileConfigInput {
                                        step: NewProfileConfigStep::ProfileName,
                                    };
                                }

                                // Clear input buffers
                                self.default_region_input = String::new();
                                self.default_output_input = String::new();
                                self.default_input_cursor = 0;
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Error saving defaults: {}", e));
                            }
                        }
                    }
                }
            }
            KeyCode::Esc => {
                self.state = AppState::Main;
                self.default_region_input = String::new();
                self.default_output_input = String::new();
                self.default_input_cursor = 0;
                self.pending_role = None;
                self.status_message = Some("Configuration cancelled".to_string());
            }
            KeyCode::Left => {
                if self.default_input_cursor > 0 {
                    self.default_input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let max_len = match current_step {
                    DefaultsConfigStep::Region => self.default_region_input.len(),
                    DefaultsConfigStep::Output => self.default_output_input.len(),
                };
                if self.default_input_cursor < max_len {
                    self.default_input_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.default_input_cursor = 0;
            }
            KeyCode::End => {
                self.default_input_cursor = match current_step {
                    DefaultsConfigStep::Region => self.default_region_input.len(),
                    DefaultsConfigStep::Output => self.default_output_input.len(),
                };
            }
            KeyCode::Backspace => {
                if self.default_input_cursor > 0 {
                    match current_step {
                        DefaultsConfigStep::Region => {
                            self.default_region_input
                                .remove(self.default_input_cursor - 1);
                        }
                        DefaultsConfigStep::Output => {
                            self.default_output_input
                                .remove(self.default_input_cursor - 1);
                        }
                    }
                    self.default_input_cursor -= 1;
                }
            }
            KeyCode::Delete => match current_step {
                DefaultsConfigStep::Region => {
                    if self.default_input_cursor < self.default_region_input.len() {
                        self.default_region_input.remove(self.default_input_cursor);
                    }
                }
                DefaultsConfigStep::Output => {
                    if self.default_input_cursor < self.default_output_input.len() {
                        self.default_output_input.remove(self.default_input_cursor);
                    }
                }
            },
            KeyCode::Char(c) => match current_step {
                DefaultsConfigStep::Region => {
                    if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
                        self.default_region_input
                            .insert(self.default_input_cursor, c);
                        self.default_input_cursor += 1;
                    }
                }
                DefaultsConfigStep::Output => {
                    if c.is_alphanumeric() || c == '-' {
                        self.default_output_input
                            .insert(self.default_input_cursor, c);
                        self.default_input_cursor += 1;
                    }
                }
            },
            _ => {}
        }
        Ok(())
    }

    async fn handle_new_profile_config_input_key(&mut self, key: KeyCode) -> Result<()> {
        let current_step = if let AppState::NewProfileConfigInput { step } = &self.state {
            step.clone()
        } else {
            return Ok(());
        };

        match key {
            KeyCode::Enter => {
                match current_step {
                    NewProfileConfigStep::ProfileName => {
                        if self.new_profile_name_input.trim().is_empty() {
                            self.status_message = Some("Profile name is required".to_string());
                        } else {
                            self.state = AppState::NewProfileConfigInput {
                                step: NewProfileConfigStep::Region,
                            };
                            self.new_profile_input_cursor = self.new_profile_region_input.len();
                        }
                    }
                    NewProfileConfigStep::Region => {
                        if self.new_profile_region_input.trim().is_empty() {
                            self.status_message = Some("Region is required".to_string());
                        } else {
                            self.state = AppState::NewProfileConfigInput {
                                step: NewProfileConfigStep::Output,
                            };
                            self.new_profile_input_cursor = self.new_profile_output_input.len();
                        }
                    }
                    NewProfileConfigStep::Output => {
                        // Save the profile with credentials
                        if let Some(account) = self.pending_role.take() {
                            let profile_name = self.new_profile_name_input.trim().to_string();
                            self.state = AppState::Loading;
                            self.save_profile_credentials(&account, &profile_name)
                                .await?;

                            // Clear input buffers
                            self.new_profile_name_input.clear();
                            self.new_profile_region_input.clear();
                            self.new_profile_output_input.clear();
                            self.new_profile_input_cursor = 0;
                        }
                    }
                }
            }
            KeyCode::Esc => {
                self.state = AppState::Main;
                self.new_profile_name_input.clear();
                self.new_profile_region_input.clear();
                self.new_profile_output_input.clear();
                self.new_profile_input_cursor = 0;
                self.pending_role = None;
                self.status_message = Some("Profile configuration cancelled".to_string());
            }
            KeyCode::Left => {
                if self.new_profile_input_cursor > 0 {
                    self.new_profile_input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let max_len = match current_step {
                    NewProfileConfigStep::ProfileName => self.new_profile_name_input.len(),
                    NewProfileConfigStep::Region => self.new_profile_region_input.len(),
                    NewProfileConfigStep::Output => self.new_profile_output_input.len(),
                };
                if self.new_profile_input_cursor < max_len {
                    self.new_profile_input_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.new_profile_input_cursor = 0;
            }
            KeyCode::End => {
                self.new_profile_input_cursor = match current_step {
                    NewProfileConfigStep::ProfileName => self.new_profile_name_input.len(),
                    NewProfileConfigStep::Region => self.new_profile_region_input.len(),
                    NewProfileConfigStep::Output => self.new_profile_output_input.len(),
                };
            }
            KeyCode::Backspace => {
                if self.new_profile_input_cursor > 0 {
                    match current_step {
                        NewProfileConfigStep::ProfileName => {
                            self.new_profile_name_input
                                .remove(self.new_profile_input_cursor - 1);
                        }
                        NewProfileConfigStep::Region => {
                            self.new_profile_region_input
                                .remove(self.new_profile_input_cursor - 1);
                        }
                        NewProfileConfigStep::Output => {
                            self.new_profile_output_input
                                .remove(self.new_profile_input_cursor - 1);
                        }
                    }
                    self.new_profile_input_cursor -= 1;
                }
            }
            KeyCode::Delete => match current_step {
                NewProfileConfigStep::ProfileName => {
                    if self.new_profile_input_cursor < self.new_profile_name_input.len() {
                        self.new_profile_name_input
                            .remove(self.new_profile_input_cursor);
                    }
                }
                NewProfileConfigStep::Region => {
                    if self.new_profile_input_cursor < self.new_profile_region_input.len() {
                        self.new_profile_region_input
                            .remove(self.new_profile_input_cursor);
                    }
                }
                NewProfileConfigStep::Output => {
                    if self.new_profile_input_cursor < self.new_profile_output_input.len() {
                        self.new_profile_output_input
                            .remove(self.new_profile_input_cursor);
                    }
                }
            },
            KeyCode::Char(c) => match current_step {
                NewProfileConfigStep::ProfileName => {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        self.new_profile_name_input
                            .insert(self.new_profile_input_cursor, c);
                        self.new_profile_input_cursor += 1;
                    }
                }
                NewProfileConfigStep::Region => {
                    if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
                        self.new_profile_region_input
                            .insert(self.new_profile_input_cursor, c);
                        self.new_profile_input_cursor += 1;
                    }
                }
                NewProfileConfigStep::Output => {
                    if c.is_alphanumeric() || c == '-' {
                        self.new_profile_output_input
                            .insert(self.new_profile_input_cursor, c);
                        self.new_profile_input_cursor += 1;
                    }
                }
            },
            _ => {}
        }
        Ok(())
    }

    async fn handle_confirmation_dialog_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // User confirmed - execute the pending action
                if let Some(action) = self.pending_confirm_action.take() {
                    match action {
                        ConfirmAction::MakeProfileDefault {
                            from_profile,
                            account: _,
                        } => {
                            // Backup existing default profile with timestamp
                            if crate::aws_config::profile_exists("default").unwrap_or(false) {
                                let backup_name = format!(
                                    "default-bak-{}",
                                    chrono::Utc::now().format("%y%m%d-%H%M%S")
                                );
                                tracing::info!(
                                    "Backing up existing default profile to '{}'",
                                    backup_name
                                );
                                if let Err(e) =
                                    crate::aws_config::rename_profile("default", &backup_name)
                                {
                                    tracing::warn!("Failed to backup default profile: {}", e);
                                }
                            }

                            // Rename the profile to default
                            match crate::aws_config::rename_profile(&from_profile, "default") {
                                Ok(()) => {
                                    self.status_message = Some(format!(
                                        "✓ Set '{}' as default profile",
                                        from_profile
                                    ));
                                    // Reload accounts to update indicators
                                    if let Err(e) = self.load_accounts().await {
                                        tracing::warn!(
                                            "Failed to reload accounts after setting default: {}",
                                            e
                                        );
                                    }
                                }
                                Err(e) => {
                                    self.status_message =
                                        Some(format!("Error setting default profile: {}", e));
                                }
                            }
                        }
                        ConfirmAction::RenameProfile {
                            old_name,
                            new_name,
                            account,
                        } => {
                            // Delete old profile if names differ
                            if old_name != new_name {
                                if let Err(e) = crate::aws_config::delete_profile(&old_name) {
                                    tracing::warn!(
                                        "Failed to delete old profile '{}': {}",
                                        old_name,
                                        e
                                    );
                                }
                            }

                            // Save the profile with new name and credentials
                            self.state = AppState::Loading;
                            self.save_profile_credentials(&account, &new_name).await?;
                        }
                        ConfirmAction::DeleteSession {
                            session_index,
                            session_name,
                        } => {
                            // Delete the session
                            if let Some(session) = self.sso_sessions.get(session_index).cloned() {
                                // Logout if active
                                if session.is_active {
                                    self.logout_session(session_index).await?;
                                }

                                // Delete from config
                                if let Err(e) = crate::aws_config::delete_sso_session(&session_name)
                                {
                                    self.status_message =
                                        Some(format!("Error deleting session: {}", e));
                                    self.state = AppState::Main;
                                    return Ok(());
                                }

                                // Remove from list
                                self.sso_sessions.remove(session_index);

                                // Update selection
                                if self.sso_sessions.is_empty() {
                                    self.sessions_list_state.select(None);
                                } else if session_index >= self.sso_sessions.len() {
                                    self.sessions_list_state
                                        .select(Some(self.sso_sessions.len() - 1));
                                }

                                self.status_message =
                                    Some(format!("✓ Deleted session '{}'", session_name));
                            }
                        }
                    }
                }
                self.state = AppState::Main;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                // User cancelled - just return to main screen
                self.pending_confirm_action = None;
                self.state = AppState::Main;
                self.status_message = Some("Action cancelled".to_string());
            }
            _ => {
                // Ignore other keys
            }
        }
        Ok(())
    }

    fn draw_confirmation_dialog(&self, f: &mut Frame, title: String, message: Vec<String>) {
        // Calculate dialog size with dynamic height
        let dialog_width = 60;

        // CRITICAL: Reserve space for essential elements
        // - borders: 2 lines
        // - title: 1 line
        // - empty after title: 1 line
        // - empty before buttons: 1 line
        // - buttons (Y/N): 1 line
        // MINIMUM dialog: 8 lines (6 fixed + at least 2 message lines)
        let min_essential_height = 8u16;

        // Get terminal dimensions
        let area = f.area();

        // Use most of the terminal height, leaving small margin
        let max_height = area.height.saturating_sub(2);

        // Calculate desired height
        let content_height = message.len() as u16;
        let desired_height = content_height + 6; // message + fixed elements

        // Final dialog height (ensure minimum)
        let dialog_height = std::cmp::max(
            min_essential_height,
            std::cmp::min(desired_height, max_height),
        );

        // Center the dialog
        let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = ratatui::layout::Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        // Calculate available space for message content
        // dialog_height - borders(2) - title(1) - empty(1) - empty(1) - buttons(1) = available
        let available_message_lines = (dialog_height as usize).saturating_sub(6).max(1);

        // Truncate message if needed - ALWAYS ensure Y/N buttons can be shown
        let message_to_show = if message.len() > available_message_lines {
            // Leave room for truncation indicator
            let truncate_at = available_message_lines.saturating_sub(1).max(1);
            let mut truncated = message[..truncate_at].to_vec();
            truncated.push("...".to_string());
            truncated
        } else {
            message
        };

        // Build dialog content
        let mut dialog_text = vec![];
        dialog_text.push(Line::from(Span::styled(
            title,
            Style::default()
                .fg(catppuccin_color(self.theme.colors.yellow))
                .add_modifier(Modifier::BOLD),
        )));
        dialog_text.push(Line::from(""));

        // Truncate long lines to fit dialog width (minus padding and borders)
        let max_line_width = (dialog_width as usize).saturating_sub(4);
        for msg in message_to_show {
            let truncated_msg = if msg.len() > max_line_width {
                format!("{}...", &msg[..max_line_width.saturating_sub(3)])
            } else {
                msg
            };
            dialog_text.push(Line::from(truncated_msg));
        }

        dialog_text.push(Line::from(""));
        dialog_text.push(Line::from(vec![
            Span::styled(
                "Y",
                Style::default()
                    .fg(catppuccin_color(self.theme.colors.green))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(": Confirm | "),
            Span::styled(
                "N",
                Style::default()
                    .fg(catppuccin_color(self.theme.colors.red))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(": Cancel"),
        ]));

        let dialog = Paragraph::new(dialog_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(catppuccin_color(self.theme.colors.yellow)))
                .title("Confirmation"),
        );
        // Note: No .wrap() - lines are pre-truncated to fit, ensuring Y/N buttons always show

        // Clear the background by rendering a clear block first
        let clear_block =
            Block::default().style(Style::default().bg(catppuccin_color(self.theme.colors.base)));
        f.render_widget(clear_block, dialog_area);

        // Render the dialog
        f.render_widget(dialog, dialog_area);
    }

    async fn save_profile_credentials(
        &mut self,
        account: &AccountRole,
        profile_name: &str,
    ) -> Result<()> {
        if let (Some(ref token), Some(ref instance)) = (&self.sso_token, &self.sso_instance) {
            // Check if target profile already exists (and is not the one being renamed)
            let target_exists = match crate::aws_config::get_profile_details(profile_name) {
                Ok(Some(_)) => {
                    // Profile exists, check if it's different from the one being renamed
                    match &self.existing_profile_name {
                        Some(existing) => existing != profile_name,
                        None => true, // No existing profile, so target is definitely different
                    }
                }
                Ok(None) => false, // Target doesn't exist
                Err(e) => {
                    tracing::warn!("Error checking if profile exists: {}", e);
                    false
                }
            };

            // If target profile exists and is different, show confirmation
            if target_exists {
                let mut message = vec![
                    format!("Profile '{}' already exists.", profile_name),
                    "".to_string(),
                ];

                // Get and display existing profile details (compact format)
                if let Ok(Some(details)) = crate::aws_config::get_profile_details(profile_name) {
                    message.push("Current profile details:".to_string());

                    // Combine region and output on one line if both exist
                    let mut settings = Vec::new();
                    if let Some(region) = details.region {
                        settings.push(format!("region={}", region));
                    }
                    if let Some(output) = details.output {
                        settings.push(format!("output={}", output));
                    }
                    if !settings.is_empty() {
                        message.push(format!("  {}", settings.join(", ")));
                    }

                    // Show SSO details if present (compact)
                    if details.sso_session.is_some()
                        || details.sso_account_id.is_some()
                        || details.sso_role_name.is_some()
                    {
                        let mut sso_parts = Vec::new();
                        if let Some(session) = details.sso_session {
                            sso_parts.push(format!("session={}", session));
                        }
                        if let Some(account_id) = details.sso_account_id {
                            sso_parts.push(format!("account={}", account_id));
                        }
                        if let Some(role) = details.sso_role_name {
                            sso_parts.push(format!("role={}", role));
                        }
                        message.push(format!("  SSO: {}", sso_parts.join(", ")));
                    }
                    message.push("".to_string());
                }

                message.push("Overwrite it?".to_string());

                // Show confirmation dialog
                let old_name = self.existing_profile_name.clone().unwrap_or_default();
                self.pending_confirm_action = Some(ConfirmAction::RenameProfile {
                    old_name,
                    new_name: profile_name.to_string(),
                    account: account.clone(),
                });
                self.state = AppState::ConfirmationDialog {
                    title: "Overwrite Existing Profile".to_string(),
                    message,
                };
                return Ok(());
            }

            self.status_message = Some(format!(
                "Getting credentials for {} / {}...",
                account.account_name, account.role_name
            ));

            // If profile name changed, delete old profile
            if let Some(ref existing) = self.existing_profile_name {
                if existing != profile_name {
                    tracing::info!(
                        "Profile name changed from '{}' to '{}', deleting old profile",
                        existing,
                        profile_name
                    );
                    if let Err(e) = crate::aws_config::delete_profile(existing) {
                        tracing::warn!("Failed to delete old profile '{}': {}", existing, e);
                    }
                }
            }

            match self
                .credential_manager
                .get_role_credentials(
                    &instance.region,
                    &token.access_token,
                    &account.account_id,
                    &account.role_name,
                )
                .await
            {
                Ok(creds) => {
                    // Use region and output from new profile config if available, otherwise defaults
                    let profile_region = if !self.new_profile_region_input.is_empty() {
                        &self.new_profile_region_input
                    } else {
                        &instance.region
                    };

                    let output_format = if !self.new_profile_output_input.is_empty() {
                        Some(self.new_profile_output_input.as_str())
                    } else {
                        sso_config::get_default_output_format()
                    };

                    // Write to AWS credentials file with metadata
                    match crate::aws_config::write_credentials_with_metadata(
                        profile_name,
                        &creds,
                        profile_region,
                        output_format,
                        Some(account),
                    ) {
                        Ok(()) => {
                            self.state = AppState::Main;
                            let mut status_msg = format!(
                                "✓ Saved profile '{}' (expires in {})",
                                profile_name,
                                creds.expiration_display()
                            );
                            if let Some(output) = output_format {
                                status_msg.push_str(&format!(" | output={}", output));
                            }
                            self.status_message = Some(status_msg);

                            // Reload accounts to update active status indicators
                            if let Err(e) = self.load_accounts().await {
                                tracing::warn!(
                                    "Failed to reload accounts after saving profile: {}",
                                    e
                                );
                            }
                        }
                        Err(e) => {
                            self.state =
                                AppState::Error(format!("Failed to write credentials: {}", e));
                        }
                    }
                }
                Err(e) => {
                    self.state = AppState::Error(format!("Failed to get credentials: {}", e));
                }
            }

            self.profile_input.clear();
            self.profile_input_cursor = 0;
            self.existing_profile_name = None;
        }
        Ok(())
    }

    #[allow(dead_code)]
    async fn login(&mut self) -> Result<()> {
        // Check if SSO config is available
        if !sso_config::has_sso_config(None, None) {
            // Show SSO configuration input screen
            self.state = AppState::SsoConfigInput {
                step: SsoConfigStep::StartUrl,
            };
            self.status_message = Some("Please configure AWS SSO to get started".to_string());
            return Ok(());
        }

        self.state = AppState::Loading;
        self.status_message = Some("Logging in to AWS SSO...".to_string());

        // Get SSO config
        let (start_url, region) = match sso_config::get_sso_config(None, None) {
            Ok(config) => config,
            Err(e) => {
                self.state = AppState::Error(format!("Config error: {}", e));
                return Ok(());
            }
        };

        // Create SSO instance
        let instance = SsoInstance {
            start_url: start_url.to_string(),
            region: region.to_string(),
            session_name: None,
        };

        // Perform login with callback to capture device auth info
        let instance_clone = instance.clone();
        match self
            .auth_manager
            .login_with_callback(&instance, false, |auth_info| {
                // Store device auth info for display in loading screen
                self.device_auth_info = Some(auth_info.clone());

                // Only try to open browser if not in headless environment
                if !crate::env::is_headless_environment() {
                    let url_to_open = auth_info
                        .verification_uri_complete
                        .as_ref()
                        .unwrap_or(&auth_info.verification_uri);

                    if let Err(e) = webbrowser::open(url_to_open) {
                        tracing::warn!("Could not open browser automatically: {}", e);
                    }
                } else {
                    tracing::info!("Headless environment detected - skipping browser launch, showing URL in TUI");
                }

                Ok(())
            })
            .await
        {
            Ok(token) => {
                tracing::info!(
                    "Login successful, token expires in {} minutes",
                    token.expires_in_minutes()
                );
                self.sso_token = Some(token);
                self.sso_instance = Some(instance_clone);
                self.device_auth_info = None; // Clear auth info
                self.state = AppState::Main;
                self.status_message = Some("Login successful! Loading accounts...".to_string());

                // Load accounts after successful login
                if let Err(e) = self.load_accounts().await {
                    self.status_message = Some(format!(
                        "Login succeeded but failed to load accounts: {}",
                        e
                    ));
                }
            }
            Err(e) => {
                tracing::error!("Login failed: {}", e);
                self.device_auth_info = None; // Clear auth info
                self.state = AppState::Error(format!("Login failed: {}", e));
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    async fn logout(&mut self) -> Result<()> {
        if let Some(ref instance) = self.sso_instance {
            // Remove cached token
            if let Err(e) = self.auth_manager.remove_token(instance) {
                tracing::warn!("Failed to remove cached token: {}", e);
            }
        }

        // Clear session data but keep profiles visible
        // (credentials may still be valid even without active SSO session)
        self.sso_token = None;
        self.sso_instance = None;
        // Don't clear accounts - profiles remain visible with their credential status
        self.status_message = Some(
            "Logged out successfully. Switch to Sessions pane (Tab) and press Enter to login."
                .to_string(),
        );

        Ok(())
    }

    /// Load all SSO sessions from ~/.aws/config and check their token status
    async fn load_all_sso_sessions(&mut self) {
        match crate::aws_config::read_all_sso_sessions() {
            Ok(sessions) => {
                tracing::info!("Loaded {} raw sessions from config", sessions.len());
                let mut sso_session_infos = Vec::new();

                for session in sessions {
                    tracing::info!(
                        "Processing session: {} ({})",
                        session.session_name,
                        session.sso_start_url
                    );
                    let instance = SsoInstance {
                        start_url: session.sso_start_url.clone(),
                        region: session.sso_region.clone(),
                        session_name: Some(session.session_name.clone()),
                    };

                    // Try to load cached token for this session
                    let (is_active, token, token_expiration) =
                        match self.auth_manager.get_cached_token(&instance) {
                            Ok(Some(token)) if !token.is_expired() => {
                                let expiration = token.expires_at;
                                (true, Some(token), Some(expiration))
                            }
                            _ => (false, None, None),
                        };

                    sso_session_infos.push(SsoSessionInfo {
                        session_name: session.session_name,
                        start_url: session.sso_start_url,
                        region: session.sso_region,
                        is_active,
                        token_expiration,
                        instance,
                        token,
                    });
                }

                self.sso_sessions = sso_session_infos;

                // Select first active session if available, otherwise select first session
                if !self.sso_sessions.is_empty() && self.sessions_list_state.selected().is_none() {
                    // Find first active session
                    let first_active_idx = self
                        .sso_sessions
                        .iter()
                        .position(|session| session.is_active);

                    let selected_idx = first_active_idx.unwrap_or(0);
                    self.sessions_list_state.select(Some(selected_idx));

                    // Set current session to the selected one if it's active
                    if let Some(selected_session) = self.sso_sessions.get(selected_idx) {
                        if selected_session.is_active {
                            self.sso_instance = Some(selected_session.instance.clone());
                            self.sso_token = selected_session.token.clone();
                        }
                    }
                }

                self.status_message =
                    Some(format!("Loaded {} SSO session(s)", self.sso_sessions.len()));
            }
            Err(e) => {
                tracing::warn!("Error loading SSO sessions: {}", e);
                self.status_message = Some(format!("Error loading sessions: {}", e));
            }
        }
    }

    #[allow(dead_code)]
    async fn load_sso_session(&mut self) {
        self.status_message = Some("Checking for existing SSO session...".to_string());

        // Check if SSO config is available
        if !sso_config::has_sso_config(None, None) {
            self.status_message = Some(
                "SSO not configured. Configure [sso-session] in ~/.aws/config or add a session using 'a'".to_string()
            );
            return;
        }

        // Get SSO config
        let (start_url, region) = match sso_config::get_sso_config(None, None) {
            Ok(config) => config,
            Err(e) => {
                self.status_message = Some(format!("Config error: {}", e));
                return;
            }
        };

        // Create SSO instance
        let instance = SsoInstance {
            start_url: start_url.to_string(),
            region: region.to_string(),
            session_name: None,
        };

        // Try to load cached token
        match self.auth_manager.get_cached_token(&instance) {
            Ok(Some(token)) => {
                if !token.is_expired() {
                    tracing::info!("Loaded valid SSO token from cache");
                    self.sso_token = Some(token);
                    self.sso_instance = Some(instance);
                    self.status_message = Some("Loaded valid SSO session from cache".to_string());

                    // Auto-load accounts
                    if let Err(e) = self.load_accounts().await {
                        self.status_message = Some(format!("Failed to load accounts: {}", e));
                    }
                } else {
                    tracing::info!("Cached SSO token has expired");
                    self.status_message =
                        Some("Cached token expired. Switch to Sessions pane (Tab) and press Enter to login.".to_string());
                }
            }
            Ok(None) => {
                tracing::info!("No cached SSO token found");
                self.status_message = Some(
                    "Not logged in. Switch to Sessions pane (Tab) and press Enter to login."
                        .to_string(),
                );
            }
            Err(e) => {
                tracing::warn!("Error loading cached token: {}", e);
                self.status_message = Some(format!("Error loading session: {}", e));
            }
        }
    }

    /// Load profiles from disk cache (for instant startup display)
    fn load_profiles_from_cache(&mut self, cache: &ProfileCache) {
        // Load SSO profiles from cache (excluding awsom-defaults which is internal)
        let mut all_profiles: Vec<ProfileEntry> = cache
            .profiles
            .iter()
            .filter(|cached| cached.profile_name != "awsom-defaults")
            .map(|cached| {
                ProfileEntry::Sso(AccountRoleWithStatus {
                    account_role: cached.account_role.clone(),
                    is_active: false, // Mark as inactive since we haven't verified
                    expiration: None,
                    is_default: cached.is_default,
                    profile_name: Some(cached.profile_name.clone()),
                })
            })
            .collect();

        // Also load static profiles from credentials file (not cached)
        // Exclude awsom-defaults which is an internal profile
        if let Ok(statuses) = crate::aws_config::list_profile_statuses() {
            for status in statuses {
                // Skip internal awsom-defaults profile
                if status.profile_name == "awsom-defaults" {
                    continue;
                }
                if status.has_credentials
                    && status.credential_type == crate::models::CredentialType::Static
                {
                    if let Some(creds) = status.static_credentials {
                        let is_default = status.profile_name == "default";
                        all_profiles.push(ProfileEntry::Static {
                            profile_name: status.profile_name,
                            is_default,
                            credentials: creds,
                        });
                    }
                }
            }
        }

        self.accounts = all_profiles;

        // Select first item if we have profiles
        if !self.accounts.is_empty() {
            self.accounts_list_state.select(Some(0));
        }

        tracing::debug!(
            "Loaded {} profiles from disk cache + static credentials",
            self.accounts.len()
        );
    }

    /// Load only static and incomplete profiles (when no SSO sessions are configured)
    fn load_static_profiles_only(&mut self) {
        let mut profiles: Vec<ProfileEntry> = Vec::new();
        let mut profiles_with_credentials: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // First, load static profiles from credentials file
        if let Ok(statuses) = crate::aws_config::list_profile_statuses() {
            for status in statuses {
                // Skip internal awsom-defaults profile
                if status.profile_name == "awsom-defaults" {
                    continue;
                }
                if status.has_credentials
                    && status.credential_type == crate::models::CredentialType::Static
                {
                    if let Some(creds) = status.static_credentials {
                        let is_default = status.profile_name == "default";
                        profiles_with_credentials.insert(status.profile_name.clone());
                        profiles.push(ProfileEntry::Static {
                            profile_name: status.profile_name,
                            is_default,
                            credentials: creds,
                        });
                    }
                }
            }
        }

        // Then, load incomplete profiles from config (profiles without credentials)
        if let Ok(config_profiles) = crate::aws_config::list_all_config_profiles() {
            for cp in config_profiles {
                // Skip if we already have credentials for this profile
                if profiles_with_credentials.contains(&cp.name) {
                    continue;
                }
                // Skip SSO profiles (they should be loaded via SSO flow)
                if cp.sso_session.is_some() || cp.sso_account_id.is_some() {
                    continue;
                }
                // This is an incomplete profile (in config but no credentials)
                profiles.push(ProfileEntry::Incomplete {
                    profile_name: cp.name,
                    region: cp.region,
                    output: cp.output,
                });
            }
        }

        self.accounts = profiles;

        tracing::debug!(
            "Loaded {} profiles (static + incomplete, no SSO sessions configured)",
            self.accounts.len()
        );
    }

    /// Save current profiles to disk cache
    fn save_profiles_to_cache(&self) {
        let cached_profiles: Vec<CachedProfile> = self
            .accounts
            .iter()
            .filter_map(|entry| {
                match entry {
                    ProfileEntry::Sso(status) => {
                        // Get session name from current session or cache
                        let session_name = self
                            .get_selected_session()
                            .map(|s| s.session_name.clone())
                            .unwrap_or_else(|| "unknown".to_string());

                        Some(CachedProfile {
                            profile_name: status.profile_name.clone().unwrap_or_else(|| {
                                format!(
                                    "{}/{}",
                                    status.account_role.account_name, status.account_role.role_name
                                )
                            }),
                            account_role: status.account_role.clone(),
                            session_name,
                            is_default: status.is_default,
                        })
                    }
                    ProfileEntry::Static { .. } => None, // Don't cache static profiles
                    ProfileEntry::Incomplete { .. } => None, // Don't cache incomplete profiles
                }
            })
            .collect();

        if !cached_profiles.is_empty() {
            if let Err(e) = cache::save_profiles(&cached_profiles) {
                tracing::warn!("Failed to save profiles to cache: {}", e);
            }
        }
    }

    /// Light-weight refresh: update credential statuses from local files
    /// without making AWS API calls. Used for auto-refresh when no SSO session is active.
    fn refresh_credential_statuses(&mut self) {
        let statuses = match crate::aws_config::list_profile_statuses() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to refresh credential statuses: {}", e);
                return;
            }
        };

        // Build a map of profile_name -> (is_active, expiration)
        let status_map: std::collections::HashMap<
            String,
            (bool, Option<chrono::DateTime<chrono::Utc>>),
        > = statuses
            .into_iter()
            .map(|s| {
                let is_active =
                    s.has_credentials && s.expiration.map_or(true, |exp| exp > chrono::Utc::now());
                (s.profile_name, (is_active, s.expiration))
            })
            .collect();

        // Update existing accounts with new statuses
        for profile in &mut self.accounts {
            match profile {
                ProfileEntry::Sso(ref mut acct) => {
                    if let Some(ref name) = acct.profile_name {
                        if let Some(&(is_active, expiration)) = status_map.get(name) {
                            acct.is_active = is_active;
                            acct.expiration = expiration;
                        }
                    }
                }
                ProfileEntry::Static { profile_name, .. } => {
                    // Static profiles don't expire, but check if they exist
                    if let Some(&(is_active, _)) = status_map.get(profile_name) {
                        // Static credentials are always active if they exist
                        let _ = is_active; // Just check presence
                    }
                }
                ProfileEntry::Incomplete { .. } => {
                    // Incomplete profiles have no credentials to refresh
                }
            }
        }

        tracing::debug!(
            "Refreshed credential statuses for {} profiles",
            self.accounts.len()
        );
    }

    /// Auto-refresh credentials that are expiring soon.
    /// This proactively refreshes credentials before they expire to maintain
    /// continuous access without user intervention.
    async fn auto_refresh_expiring_credentials(&mut self, threshold_minutes: i64) {
        // Need both SSO token and instance to refresh credentials
        let (token, instance) = match (&self.sso_token, &self.sso_instance) {
            (Some(t), Some(i)) => (t.clone(), i.clone()),
            _ => return,
        };

        // Don't refresh if SSO token itself is expired
        if token.is_expired() {
            return;
        }

        let now = chrono::Utc::now();

        // Collect profiles that need credential refresh
        let profiles_to_refresh: Vec<(AccountRole, String)> = self
            .accounts
            .iter()
            .filter_map(|profile| {
                if let ProfileEntry::Sso(ref status) = profile {
                    // Only refresh if:
                    // 1. Has an active profile name
                    // 2. Has an expiration time
                    // 3. Is expiring within threshold
                    // 4. Is currently active (has valid credentials)
                    if let (Some(ref profile_name), Some(expiration)) =
                        (&status.profile_name, status.expiration)
                    {
                        if status.is_active {
                            let remaining_minutes = (expiration - now).num_minutes();
                            if remaining_minutes > 0 && remaining_minutes <= threshold_minutes {
                                return Some((status.account_role.clone(), profile_name.clone()));
                            }
                        }
                    }
                }
                None
            })
            .collect();

        if profiles_to_refresh.is_empty() {
            return;
        }

        tracing::info!(
            "Auto-refreshing {} credential(s) expiring within {} minutes",
            profiles_to_refresh.len(),
            threshold_minutes
        );

        let mut refreshed_count = 0;

        for (account, profile_name) in profiles_to_refresh {
            tracing::debug!(
                "Refreshing credentials for {} ({}/{})",
                profile_name,
                account.account_name,
                account.role_name
            );

            match self
                .credential_manager
                .get_role_credentials(
                    &instance.region,
                    &token.access_token,
                    &account.account_id,
                    &account.role_name,
                )
                .await
            {
                Ok(new_creds) => {
                    // Write refreshed credentials to ~/.aws/credentials
                    if let Err(e) = crate::aws_config::write_credentials_with_metadata(
                        &profile_name,
                        &new_creds,
                        &instance.region,
                        None,
                        Some(&account),
                    ) {
                        tracing::warn!(
                            "Failed to write refreshed credentials for '{}': {}",
                            profile_name,
                            e
                        );
                        continue;
                    }

                    // Update the in-memory status
                    for profile in &mut self.accounts {
                        if let ProfileEntry::Sso(ref mut status) = profile {
                            if status.profile_name.as_ref() == Some(&profile_name) {
                                status.expiration = Some(new_creds.expiration);
                                status.is_active = true;
                            }
                        }
                    }

                    refreshed_count += 1;
                    tracing::info!(
                        "✓ Refreshed credentials for '{}' (expires in {})",
                        profile_name,
                        new_creds.expiration_display()
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to refresh credentials for '{}': {}",
                        profile_name,
                        e
                    );
                }
            }
        }

        if refreshed_count > 0 {
            self.status_message = Some(format!(
                "✓ Auto-refreshed {} credential(s)",
                refreshed_count
            ));
        }
    }

    async fn load_accounts(&mut self) -> Result<()> {
        // Determine which session's accounts to show
        // Priority: filtered_session > currently selected session
        let _target_session = if let Some(ref filtered) = self.filtered_session {
            Some(filtered.clone())
        } else {
            self.get_selected_session()
                .map(|selected_session| selected_session.session_name.clone())
        };

        // Check if we have an active SSO session with token
        let _has_active_session = self.sso_token.is_some() && self.sso_instance.is_some();

        // Get current session name (for caching when fetching from AWS)
        let current_session_name = self
            .get_selected_session()
            .map(|selected_session| selected_session.session_name.clone());

        if let (Some(ref token), Some(ref instance)) = (&self.sso_token, &self.sso_instance) {
            // We have an active SSO session

            // If filtering is active but it's not the current session, use cache only
            if let Some(ref filtered) = self.filtered_session {
                if Some(filtered.clone()) != current_session_name {
                    // Filtering a different session - use cache only
                    if let Some(cached_roles) = self.accounts_cache.get(filtered) {
                        tracing::debug!(
                            "Showing cached accounts for filtered session: {}",
                            filtered
                        );
                        // Build account list from cached roles (mark as inactive since not current session)
                        let sso_profiles: Vec<ProfileEntry> = cached_roles
                            .iter()
                            .map(|account_role| {
                                ProfileEntry::Sso(AccountRoleWithStatus {
                                    account_role: account_role.clone(),
                                    is_active: false,
                                    expiration: None,
                                    is_default: false,
                                    profile_name: None,
                                })
                            })
                            .collect();

                        self.accounts = sso_profiles;
                        self.state = AppState::Main;
                        self.status_message = Some(format!(
                            "Filtered: showing {} accounts from session '{}'",
                            self.accounts.len(),
                            filtered
                        ));

                        // Select first item if none selected
                        if self.accounts_list_state.selected().is_none()
                            && !self.accounts.is_empty()
                        {
                            self.accounts_list_state.select(Some(0));
                        }
                        return Ok(());
                    } else {
                        // No cache for filtered session - show empty with message
                        tracing::debug!("No cached accounts for filtered session: {}", filtered);
                        self.accounts = Vec::new();
                        self.state = AppState::Main;
                        self.status_message = Some(format!(
                            "No cached data for session '{}'. Press 'f' again to unfilter.",
                            filtered
                        ));
                        return Ok(());
                    }
                }
            }

            // Either no filter, or filtering the current active session - fetch/cache normally
            self.state = AppState::Loading;
            self.status_message = Some("Loading accounts and roles...".to_string());

            // Try to get from cache first
            let mut all_roles = if let Some(ref sess_name) = current_session_name {
                if let Some(cached_roles) = self.accounts_cache.get(sess_name) {
                    // Use cached roles
                    tracing::debug!("Using cached accounts for session: {}", sess_name);
                    cached_roles.clone()
                } else {
                    // Fetch from AWS and cache
                    Vec::new() // Will be populated below
                }
            } else {
                Vec::new() // No session, will fetch fresh
            };

            // Fetch from AWS if not in cache
            if all_roles.is_empty() {
                match self
                    .credential_manager
                    .list_accounts(&instance.region, &token.access_token)
                    .await
                {
                    Ok(account_list) => {
                        // Now fetch roles for each account
                        for (account_id, account_name) in account_list {
                            match self
                                .credential_manager
                                .list_account_roles(
                                    &instance.region,
                                    &token.access_token,
                                    &account_id,
                                )
                                .await
                            {
                                Ok(roles) => {
                                    for role_name in roles {
                                        all_roles.push(AccountRole {
                                            account_id: account_id.clone(),
                                            account_name: account_name.clone(),
                                            role_name,
                                        });
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to list roles for account {}: {}",
                                        account_id,
                                        e
                                    );
                                }
                            }
                        }

                        // Cache the fetched roles for this session
                        if let Some(ref sess_name) = current_session_name {
                            tracing::debug!(
                                "Caching {} accounts/roles for session: {}",
                                all_roles.len(),
                                sess_name
                            );
                            self.accounts_cache
                                .insert(sess_name.clone(), all_roles.clone());
                        }
                    }
                    Err(e) => {
                        self.state = AppState::Error(format!("Failed to load accounts: {}", e));
                        return Ok(());
                    }
                }
            }

            // Load credential statuses from AWS config
            let statuses = crate::aws_config::list_profile_statuses().unwrap_or_default();

            // Build a map from (account_id, role_name) to (is_active, expiration, is_default)
            #[allow(clippy::type_complexity)]
            let mut profile_map: HashMap<
                (String, String),
                (bool, Option<chrono::DateTime<chrono::Utc>>, bool),
            > = HashMap::new();

            // Collect static profiles separately
            let mut static_profiles: Vec<ProfileEntry> = Vec::new();

            for status in statuses {
                // Skip internal awsom-defaults profile
                if status.profile_name == "awsom-defaults" {
                    continue;
                }
                if status.has_credentials {
                    match status.credential_type {
                        crate::models::CredentialType::Sso => {
                            if let (Some(account_id), Some(role_name)) =
                                (status.account_id, status.role_name)
                            {
                                // Check if this is the default profile
                                let is_default = status.profile_name == "default";

                                // Check if credentials are expired
                                let is_active = if let Some(expiration) = status.expiration {
                                    chrono::Utc::now() < expiration
                                } else {
                                    // No expiration info means credentials exist but we can't verify validity
                                    true
                                };

                                // Match by account ID and role name from metadata
                                profile_map.insert(
                                    (account_id, role_name),
                                    (is_active, status.expiration, is_default),
                                );
                            }
                        }
                        crate::models::CredentialType::Static => {
                            // Add static profile entry
                            if let Some(creds) = status.static_credentials {
                                let is_default = status.profile_name == "default";
                                static_profiles.push(ProfileEntry::Static {
                                    profile_name: status.profile_name,
                                    is_default,
                                    credentials: creds,
                                });
                            }
                        }
                    }
                }
            }

            // Wrap SSO roles with status
            let sso_profiles: Vec<ProfileEntry> = all_roles
                .into_iter()
                .map(|account_role| {
                    // Match by account ID and role name
                    let key = (
                        account_role.account_id.clone(),
                        account_role.role_name.clone(),
                    );
                    let (is_active, expiration, is_default) = profile_map
                        .get(&key)
                        .cloned()
                        .unwrap_or((false, None, false));

                    // Look up profile name using unified lookup
                    let profile_name = if let Some(ref sess_name) = current_session_name {
                        crate::aws_config::get_profile_by_role(
                            sess_name,
                            &account_role.account_id,
                            &account_role.role_name,
                        )
                        .ok()
                        .flatten()
                        .map(|p| p.name)
                    } else {
                        None
                    };

                    ProfileEntry::Sso(AccountRoleWithStatus {
                        account_role,
                        is_active,
                        expiration,
                        is_default,
                        profile_name,
                    })
                })
                .collect();

            // Also load profiles from config file that may not be in AWS API results
            // These are profiles that have been previously configured but the account/role
            // might not appear in the current list (e.g., different SSO session)
            let config_profiles = crate::aws_config::list_sso_profiles().unwrap_or_default();

            // Create a set of (account_id, role_name) from AWS API results for deduplication
            let api_keys: std::collections::HashSet<(String, String)> = sso_profiles
                .iter()
                .filter_map(|p| {
                    if let ProfileEntry::Sso(status) = p {
                        Some((
                            status.account_role.account_id.clone(),
                            status.account_role.role_name.clone(),
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            // Add config profiles that aren't already in the list
            let mut config_entries: Vec<ProfileEntry> = Vec::new();
            for config_profile in config_profiles {
                // Skip internal awsom-defaults profile
                if config_profile.name == "awsom-defaults" {
                    continue;
                }
                // Skip if this profile is already in API results
                if let (Some(ref account_id), Some(ref role_name)) = (
                    &config_profile.sso_account_id,
                    &config_profile.sso_role_name,
                ) {
                    if api_keys.contains(&(account_id.clone(), role_name.clone())) {
                        continue;
                    }

                    // Check if this profile's session matches current session
                    let is_current_session = if let (Some(ref config_sess), Some(ref curr_sess)) =
                        (&config_profile.sso_session, &current_session_name)
                    {
                        config_sess == curr_sess
                    } else {
                        false
                    };

                    // Only show profiles from current session or profiles without session specified
                    if is_current_session || config_profile.sso_session.is_none() {
                        // Look up credential status for this profile
                        let key = (account_id.clone(), role_name.clone());
                        let (is_active, expiration, is_default) = profile_map
                            .get(&key)
                            .cloned()
                            .unwrap_or((false, None, config_profile.name == "default"));

                        config_entries.push(ProfileEntry::Sso(AccountRoleWithStatus {
                            account_role: crate::models::AccountRole {
                                account_id: account_id.clone(),
                                account_name: format!("(from config: {})", config_profile.name),
                                role_name: role_name.clone(),
                            },
                            is_active,
                            expiration,
                            is_default,
                            profile_name: Some(config_profile.name),
                        }));
                    }
                }
            }

            // Combine SSO and static profiles
            let mut all_profiles: Vec<ProfileEntry> = sso_profiles;
            all_profiles.extend(config_entries);
            all_profiles.extend(static_profiles);

            // Also load incomplete profiles (config without credentials)
            // First, build a set of profile names that already have credentials
            let profiles_with_creds: std::collections::HashSet<String> = all_profiles
                .iter()
                .filter_map(|p| match p {
                    ProfileEntry::Sso(status) => status.profile_name.clone(),
                    ProfileEntry::Static { profile_name, .. } => Some(profile_name.clone()),
                    ProfileEntry::Incomplete { .. } => None,
                })
                .collect();

            // Add incomplete profiles from config
            if let Ok(config_profiles) = crate::aws_config::list_all_config_profiles() {
                for cp in config_profiles {
                    // Skip if we already have credentials for this profile
                    if profiles_with_creds.contains(&cp.name) {
                        continue;
                    }
                    // Skip SSO profiles (they should appear via SSO flow)
                    if cp.sso_session.is_some() || cp.sso_account_id.is_some() {
                        continue;
                    }
                    // This is an incomplete profile
                    all_profiles.push(ProfileEntry::Incomplete {
                        profile_name: cp.name,
                        region: cp.region,
                        output: cp.output,
                    });
                }
            }

            // Sort: SSO profiles by account/role name, static/incomplete profiles by profile name
            all_profiles.sort_by(|a, b| {
                match (a, b) {
                    (ProfileEntry::Sso(a_status), ProfileEntry::Sso(b_status)) => a_status
                        .account_role
                        .account_name
                        .cmp(&b_status.account_role.account_name)
                        .then_with(|| {
                            a_status
                                .account_role
                                .role_name
                                .cmp(&b_status.account_role.role_name)
                        }),
                    (
                        ProfileEntry::Static {
                            profile_name: a_name,
                            ..
                        },
                        ProfileEntry::Static {
                            profile_name: b_name,
                            ..
                        },
                    ) => a_name.cmp(b_name),
                    (
                        ProfileEntry::Incomplete {
                            profile_name: a_name,
                            ..
                        },
                        ProfileEntry::Incomplete {
                            profile_name: b_name,
                            ..
                        },
                    ) => a_name.cmp(b_name),
                    // Sort order: SSO < Static < Incomplete
                    (ProfileEntry::Sso(_), _) => std::cmp::Ordering::Less,
                    (_, ProfileEntry::Sso(_)) => std::cmp::Ordering::Greater,
                    (ProfileEntry::Static { .. }, ProfileEntry::Incomplete { .. }) => {
                        std::cmp::Ordering::Less
                    }
                    (ProfileEntry::Incomplete { .. }, ProfileEntry::Static { .. }) => {
                        std::cmp::Ordering::Greater
                    }
                }
            });

            self.accounts = all_profiles;
            self.state = AppState::Main;
            self.status_message = Some(format!(
                "Loaded {} account/role combinations",
                self.accounts.len()
            ));

            // Save to disk cache and clear cached data flag (now showing fresh data)
            self.save_profiles_to_cache();
            self.showing_cached_data = None;

            // Select first item if none selected
            if self.accounts_list_state.selected().is_none() && !self.accounts.is_empty() {
                self.accounts_list_state.select(Some(0));
            }
        } else {
            // No active SSO session - show only static profiles (and optionally cached accounts)
            tracing::debug!("No active SSO session - clearing SSO accounts");

            // Load credential statuses to get static profiles
            let statuses = crate::aws_config::list_profile_statuses().unwrap_or_default();

            let mut all_profiles: Vec<ProfileEntry> = Vec::new();

            // Add static profiles (excluding internal awsom-defaults)
            for status in &statuses {
                // Skip internal awsom-defaults profile
                if status.profile_name == "awsom-defaults" {
                    continue;
                }
                if status.has_credentials
                    && status.credential_type == crate::models::CredentialType::Static
                {
                    if let Some(ref creds) = status.static_credentials {
                        let is_default = status.profile_name == "default";
                        all_profiles.push(ProfileEntry::Static {
                            profile_name: status.profile_name.clone(),
                            is_default,
                            credentials: creds.clone(),
                        });
                    }
                }
            }

            // If there's a filtered session, show its cached accounts (marked as inactive)
            if let Some(ref filtered_sess) = self.filtered_session {
                if let Some(cached_roles) = self.accounts_cache.get(filtered_sess) {
                    tracing::debug!(
                        "Adding {} cached accounts from filtered session: {}",
                        cached_roles.len(),
                        filtered_sess
                    );

                    // Add cached SSO accounts marked as inactive
                    for account_role in cached_roles {
                        all_profiles.push(ProfileEntry::Sso(AccountRoleWithStatus {
                            account_role: account_role.clone(),
                            is_active: false,
                            expiration: None,
                            is_default: false,
                            profile_name: None,
                        }));
                    }
                }
            }

            // Also load SSO profiles from config file
            let config_profiles = crate::aws_config::list_sso_profiles().unwrap_or_default();

            // Create a set of existing (account_id, role_name) for deduplication
            let existing_keys: std::collections::HashSet<(String, String)> = all_profiles
                .iter()
                .filter_map(|p| {
                    if let ProfileEntry::Sso(status) = p {
                        Some((
                            status.account_role.account_id.clone(),
                            status.account_role.role_name.clone(),
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            // Build profile status map from credentials
            #[allow(clippy::type_complexity)]
            let mut profile_map: std::collections::HashMap<
                (String, String),
                (bool, Option<chrono::DateTime<chrono::Utc>>, bool),
            > = std::collections::HashMap::new();

            for status in &statuses {
                if status.has_credentials
                    && status.credential_type == crate::models::CredentialType::Sso
                {
                    if let (Some(account_id), Some(role_name)) =
                        (&status.account_id, &status.role_name)
                    {
                        let is_default = status.profile_name == "default";
                        let is_active = if let Some(exp) = status.expiration {
                            chrono::Utc::now() < exp
                        } else {
                            false
                        };
                        profile_map.insert(
                            (account_id.clone(), role_name.clone()),
                            (is_active, status.expiration, is_default),
                        );
                    }
                }
            }

            // Add config profiles that aren't already in the list
            for config_profile in config_profiles {
                // Skip internal awsom-defaults profile
                if config_profile.name == "awsom-defaults" {
                    continue;
                }
                if let (Some(ref account_id), Some(ref role_name)) = (
                    &config_profile.sso_account_id,
                    &config_profile.sso_role_name,
                ) {
                    if existing_keys.contains(&(account_id.clone(), role_name.clone())) {
                        continue;
                    }

                    // Look up credential status for this profile
                    let key = (account_id.clone(), role_name.clone());
                    let (is_active, expiration, is_default) = profile_map
                        .get(&key)
                        .cloned()
                        .unwrap_or((false, None, config_profile.name == "default"));

                    all_profiles.push(ProfileEntry::Sso(AccountRoleWithStatus {
                        account_role: crate::models::AccountRole {
                            account_id: account_id.clone(),
                            account_name: format!("(from config: {})", config_profile.name),
                            role_name: role_name.clone(),
                        },
                        is_active,
                        expiration,
                        is_default,
                        profile_name: Some(config_profile.name),
                    }));
                }
            }

            // Also load incomplete profiles (config without credentials)
            let profiles_with_creds: std::collections::HashSet<String> = all_profiles
                .iter()
                .filter_map(|p| match p {
                    ProfileEntry::Sso(status) => status.profile_name.clone(),
                    ProfileEntry::Static { profile_name, .. } => Some(profile_name.clone()),
                    ProfileEntry::Incomplete { .. } => None,
                })
                .collect();

            if let Ok(config_profiles) = crate::aws_config::list_all_config_profiles() {
                for cp in config_profiles {
                    if profiles_with_creds.contains(&cp.name) {
                        continue;
                    }
                    if cp.sso_session.is_some() || cp.sso_account_id.is_some() {
                        continue;
                    }
                    all_profiles.push(ProfileEntry::Incomplete {
                        profile_name: cp.name,
                        region: cp.region,
                        output: cp.output,
                    });
                }
            }

            // Sort profiles
            all_profiles.sort_by(|a, b| match (a, b) {
                (ProfileEntry::Sso(a_status), ProfileEntry::Sso(b_status)) => a_status
                    .account_role
                    .account_name
                    .cmp(&b_status.account_role.account_name)
                    .then_with(|| {
                        a_status
                            .account_role
                            .role_name
                            .cmp(&b_status.account_role.role_name)
                    }),
                (
                    ProfileEntry::Static {
                        profile_name: a_name,
                        ..
                    },
                    ProfileEntry::Static {
                        profile_name: b_name,
                        ..
                    },
                ) => a_name.cmp(b_name),
                (
                    ProfileEntry::Incomplete {
                        profile_name: a_name,
                        ..
                    },
                    ProfileEntry::Incomplete {
                        profile_name: b_name,
                        ..
                    },
                ) => a_name.cmp(b_name),
                // Sort order: SSO < Static < Incomplete
                (ProfileEntry::Sso(_), _) => std::cmp::Ordering::Less,
                (_, ProfileEntry::Sso(_)) => std::cmp::Ordering::Greater,
                (ProfileEntry::Static { .. }, ProfileEntry::Incomplete { .. }) => {
                    std::cmp::Ordering::Less
                }
                (ProfileEntry::Incomplete { .. }, ProfileEntry::Static { .. }) => {
                    std::cmp::Ordering::Greater
                }
            });

            self.accounts = all_profiles;

            if self.accounts.is_empty() {
                self.status_message = Some(
                    "No active session. Switch to Sessions pane (Tab) and press Enter to login."
                        .to_string(),
                );
            } else {
                self.status_message = Some(format!(
                    "Showing {} profiles (no active SSO session)",
                    self.accounts.len()
                ));
            }

            // Select first item if none selected
            if self.accounts_list_state.selected().is_none() && !self.accounts.is_empty() {
                self.accounts_list_state.select(Some(0));
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    async fn get_credentials_for_role(&mut self, account: &AccountRole) -> Result<()> {
        if let (Some(ref token), Some(ref instance)) = (&self.sso_token, &self.sso_instance) {
            self.status_message = Some(format!(
                "Getting credentials for {} / {}...",
                account.account_name, account.role_name
            ));

            match self
                .credential_manager
                .get_role_credentials(
                    &instance.region,
                    &token.access_token,
                    &account.account_id,
                    &account.role_name,
                )
                .await
            {
                Ok(creds) => {
                    self.status_message = Some(format!(
                        "Credentials cached for {} / {} (expires in {})",
                        account.account_name,
                        account.role_name,
                        creds.expiration_display()
                    ));
                }
                Err(e) => {
                    self.status_message = Some(format!("Error: {}", e));
                }
            }
        }
        Ok(())
    }

    /// View profile details for selected profile
    fn view_profile_details(&mut self) {
        if let Some(index) = self.accounts_list_state.selected() {
            if let Some(profile_entry) = self.accounts.get(index) {
                let mut details: Vec<(String, String)> = Vec::new();

                match profile_entry {
                    ProfileEntry::Sso(status) => {
                        // Profile name
                        if let Some(ref name) = status.profile_name {
                            details.push(("Profile".to_string(), name.clone()));
                        }

                        // Account info
                        details.push((
                            "Account".to_string(),
                            format!(
                                "{} ({})",
                                status.account_role.account_name, status.account_role.account_id
                            ),
                        ));
                        details.push(("Role".to_string(), status.account_role.role_name.clone()));

                        // Look up additional details from config
                        if let Some(ref profile_name) = status.profile_name {
                            if let Ok(Some(config_details)) =
                                crate::aws_config::get_profile_details(profile_name)
                            {
                                if let Some(region) = config_details.region {
                                    details.push(("Region".to_string(), region));
                                }
                                if let Some(output) = config_details.output {
                                    details.push(("Output".to_string(), output));
                                }
                                if let Some(sso_session) = config_details.sso_session {
                                    details.push(("SSO Session".to_string(), sso_session));
                                }
                            }
                        }

                        // Status
                        if status.is_active {
                            if let Some(exp) = status.expiration {
                                let remaining = exp - chrono::Utc::now();
                                let mins = remaining.num_minutes();
                                details.push(("Status".to_string(), format!("Active ({}m)", mins)));
                            } else {
                                details.push(("Status".to_string(), "Active".to_string()));
                            }
                        } else {
                            details.push(("Status".to_string(), "Inactive".to_string()));
                        }

                        if status.is_default {
                            details.push(("Default".to_string(), "Yes".to_string()));
                        }
                    }
                    ProfileEntry::Static {
                        profile_name,
                        is_default,
                        ..
                    } => {
                        details.push(("Profile".to_string(), profile_name.clone()));
                        details.push(("Type".to_string(), "Static credentials".to_string()));

                        // Look up additional details from config
                        if let Ok(Some(config_details)) =
                            crate::aws_config::get_profile_details(profile_name)
                        {
                            if let Some(region) = config_details.region {
                                details.push(("Region".to_string(), region));
                            }
                            if let Some(output) = config_details.output {
                                details.push(("Output".to_string(), output));
                            }
                        }

                        if *is_default {
                            details.push(("Default".to_string(), "Yes".to_string()));
                        }
                    }
                    ProfileEntry::Incomplete {
                        profile_name,
                        region,
                        output,
                    } => {
                        details.push(("Profile".to_string(), profile_name.clone()));
                        details.push((
                            "Type".to_string(),
                            "Incomplete (no credentials)".to_string(),
                        ));

                        if let Some(ref r) = region {
                            details.push(("Region".to_string(), r.clone()));
                        }
                        if let Some(ref o) = output {
                            details.push(("Output".to_string(), o.clone()));
                        }

                        details.push(("Status".to_string(), "No credentials".to_string()));
                    }
                }

                self.state = AppState::ViewProfile { details };
            }
        }
    }

    /// Open AWS Console in browser for selected role
    async fn open_console(&mut self) -> Result<()> {
        if let Some(index) = self.accounts_list_state.selected() {
            if let Some(profile_entry) = self.accounts.get(index).cloned() {
                // Only SSO profiles support console access
                let account_with_status = match profile_entry {
                    ProfileEntry::Sso(status) => status,
                    ProfileEntry::Static { .. } => {
                        self.status_message =
                            Some("Console access is only available for SSO profiles".to_string());
                        return Ok(());
                    }
                    ProfileEntry::Incomplete { profile_name, .. } => {
                        self.status_message = Some(format!(
                            "Profile '{}' has no credentials. Cannot open console.",
                            profile_name
                        ));
                        return Ok(());
                    }
                };

                let account = account_with_status.account_role;

                // Check if credentials are active
                if !account_with_status.is_active {
                    self.status_message = Some("No active credentials for this role. Press Enter to create credentials first.".to_string());
                    return Ok(());
                }

                // Get credentials to open console
                if let (Some(ref token), Some(ref instance)) = (&self.sso_token, &self.sso_instance)
                {
                    self.status_message = Some("Opening AWS Console in browser...".to_string());

                    match self
                        .credential_manager
                        .get_role_credentials(
                            &instance.region,
                            &token.access_token,
                            &account.account_id,
                            &account.role_name,
                        )
                        .await
                    {
                        Ok(creds) => {
                            // Use SSO region as default
                            let region = Some(instance.region.as_str());

                            match crate::console::open_console(&creds, region) {
                                Ok(()) => {
                                    self.status_message = Some(format!(
                                        "✓ Opened AWS Console for {} / {}",
                                        account.account_name, account.role_name
                                    ));
                                }
                                Err(e) => {
                                    self.status_message =
                                        Some(format!("Error opening console: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Error getting credentials: {}", e));
                        }
                    }
                }
            }
        } else {
            self.status_message = Some("No role selected".to_string());
        }
        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        // Note: draw_loading_screen needs &mut self to poll device_auth_info from Arc
        match &self.state {
            AppState::Main => self.draw_main_screen(f),
            AppState::Help => self.draw_help_screen(f),
            AppState::Loading => self.draw_loading_screen(f),
            AppState::Error(msg) => self.draw_error_screen(f, msg.clone()),
            AppState::ProfileInput => self.draw_profile_input_screen(f),
            AppState::SsoConfigInput { step } => self.draw_sso_config_input_screen(f, step.clone()),
            AppState::DefaultsConfigInput { step } => {
                self.draw_defaults_config_input_screen(f, step.clone())
            }
            AppState::NewProfileConfigInput { step } => {
                self.draw_new_profile_config_input_screen(f, step.clone())
            }
            AppState::StaticCredentialInput { step } => {
                self.draw_static_credential_input_screen(f, step.clone())
            }
            AppState::ConfirmationDialog { title, message } => {
                self.draw_confirmation_dialog(f, title.clone(), message.clone())
            }
            AppState::ViewProfile { details } => self.draw_view_profile_screen(f, details.clone()),
            AppState::SsmBrowser => self.draw_ssm_browser(f),
            AppState::ViewInstanceTags { tags } => {
                self.draw_view_instance_tags_screen(f, tags.clone())
            }
        }
    }

    fn draw_main_screen(&mut self, f: &mut Frame) {
        // Calculate dynamic sessions pane height
        // Min 5 lines (1 border top + 1 header + 1 header margin + 1 content + 1 border bottom)
        // Max 12 lines to avoid taking too much space
        let sessions_count = self.sso_sessions.len();
        let sessions_height = if sessions_count == 0 {
            5 // Minimum height for empty pane
        } else {
            // 4 for borders + header + header margin, plus 1 line per session, max 12 total
            std::cmp::min(sessions_count + 4, 12)
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                      // Header
                Constraint::Min(10),                        // Accounts pane (flexible)
                Constraint::Length(sessions_height as u16), // Sessions pane (dynamic)
                Constraint::Length(2),                      // Help bar (2 lines)
            ])
            .split(f.area());

        // Header with optional status message
        let header_text = if let Some(ref msg) = self.status_message {
            // Add spinner if message indicates loading
            let spinner_frames: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let is_loading =
                msg.contains("Loading") || msg.contains("Refreshing") || msg.contains("...");
            if is_loading {
                let spinner = spinner_frames[self.tick_count as usize % spinner_frames.len()];
                format!("awsom - {} {}", spinner, msg)
            } else {
                format!("awsom - {}", msg)
            }
        } else {
            "awsom - AWS Organization Manager".to_string()
        };
        let header = Paragraph::new(header_text)
            .style(
                Style::default()
                    .fg(catppuccin_color(self.theme.colors.blue))
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Account/Role table
        let rows: Vec<Row> = self
            .accounts
            .iter()
            .map(|profile_entry| {
                match profile_entry {
                    ProfileEntry::Sso(account_with_status) => {
                        let account = &account_with_status.account_role;

                        // Default marker
                        let default_mark = if account_with_status.is_default {
                            "✓"
                        } else {
                            ""
                        };

                        // Calculate expiration status and actual active state
                        let (is_actually_active, expiration_status) =
                            if account_with_status.is_active {
                                if let Some(expiration) = account_with_status.expiration {
                                    let now = chrono::Utc::now();
                                    let remaining_secs = (expiration - now).num_seconds();

                                    if remaining_secs > 0 {
                                        let hours = remaining_secs / 3600;
                                        let mins = (remaining_secs % 3600) / 60;

                                        let display = if hours > 0 {
                                            format!("{}h {}m", hours, mins)
                                        } else {
                                            format!("{}m", mins)
                                        };
                                        (true, display)
                                    } else {
                                        (false, "EXPIRED".to_string())
                                    }
                                } else {
                                    (true, "".to_string())
                                }
                            } else {
                                (false, "".to_string())
                            };

                        // Status indicator based on actual expiration state
                        let status = if is_actually_active { "🟢" } else { "🔴" };

                        // Profile name or "N/A"
                        let profile_display =
                            account_with_status.profile_name.as_deref().unwrap_or("N/A");

                        Row::new(vec![
                            Cell::new(Text::from("SSO").alignment(Alignment::Center)),
                            Cell::new(Text::from(status).alignment(Alignment::Center)),
                            Cell::new(Text::from(default_mark).alignment(Alignment::Center)),
                            Cell::new(
                                Text::from(account.account_name.clone())
                                    .alignment(Alignment::Center),
                            ),
                            Cell::new(
                                Text::from(account.account_id.clone()).alignment(Alignment::Center),
                            ),
                            Cell::new(
                                Text::from(account.role_name.clone()).alignment(Alignment::Center),
                            ),
                            Cell::new(Text::from(profile_display).alignment(Alignment::Center)),
                            Cell::new(Text::from(expiration_status).alignment(Alignment::Center)),
                        ])
                    }
                    ProfileEntry::Static {
                        profile_name,
                        is_default,
                        ..
                    } => {
                        // Default marker
                        let default_mark = if *is_default { "✓" } else { "" };

                        // Static credentials are always active (no expiration)
                        let status = "🟢";

                        Row::new(vec![
                            Cell::new(Text::from("STATIC").alignment(Alignment::Center)),
                            Cell::new(Text::from(status).alignment(Alignment::Center)),
                            Cell::new(Text::from(default_mark).alignment(Alignment::Center)),
                            Cell::new(Text::from("-").alignment(Alignment::Center)),
                            Cell::new(Text::from("-").alignment(Alignment::Center)),
                            Cell::new(Text::from("-").alignment(Alignment::Center)),
                            Cell::new(
                                Text::from(profile_name.clone()).alignment(Alignment::Center),
                            ),
                            Cell::new(Text::from("-").alignment(Alignment::Center)),
                        ])
                    }
                    ProfileEntry::Incomplete { profile_name, .. } => {
                        // Incomplete profiles have no credentials
                        let status = "⚠";

                        Row::new(vec![
                            Cell::new(Text::from("CONFIG").alignment(Alignment::Center)),
                            Cell::new(Text::from(status).alignment(Alignment::Center)),
                            Cell::new(Text::from("").alignment(Alignment::Center)),
                            Cell::new(Text::from("-").alignment(Alignment::Center)),
                            Cell::new(Text::from("-").alignment(Alignment::Center)),
                            Cell::new(Text::from("-").alignment(Alignment::Center)),
                            Cell::new(
                                Text::from(profile_name.clone()).alignment(Alignment::Center),
                            ),
                            Cell::new(Text::from("NO CREDS").alignment(Alignment::Center)),
                        ])
                    }
                }
            })
            .collect();

        let header = Row::new(vec![
            Cell::new(Text::from("Type").alignment(Alignment::Center)),
            Cell::new(Text::from("Status").alignment(Alignment::Center)),
            Cell::new(Text::from("Default").alignment(Alignment::Center)),
            Cell::new(Text::from("Account").alignment(Alignment::Center)),
            Cell::new(Text::from("Account ID").alignment(Alignment::Center)),
            Cell::new(Text::from("Role").alignment(Alignment::Center)),
            Cell::new(Text::from("Profile").alignment(Alignment::Center)),
            Cell::new(Text::from("Expires").alignment(Alignment::Center)),
        ])
        .style(
            Style::default()
                .fg(catppuccin_color(self.theme.colors.blue))
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

        // Highlight accounts pane if it's active
        let accounts_block_style = if self.active_pane == ActivePane::Accounts {
            Style::default().fg(catppuccin_color(self.theme.colors.mauve))
        } else {
            Style::default().fg(catppuccin_color(self.theme.colors.surface0))
        };

        // Add asterisk to title if this pane is active, show filter/cache status
        let cache_indicator = self
            .showing_cached_data
            .as_ref()
            .map(|c| format!(" [cached: {}]", c.age_display()))
            .unwrap_or_default();

        let accounts_title = if let Some(ref filtered) = self.filtered_session {
            // Show filter status in title
            if self.active_pane == ActivePane::Accounts {
                format!(
                    "Profiles & Roles (*) [Filtered: {}]{}",
                    filtered, cache_indicator
                )
            } else {
                format!(
                    "Profiles & Roles [Filtered: {}]{}",
                    filtered, cache_indicator
                )
            }
        } else if self.active_pane == ActivePane::Accounts {
            format!("Profiles & Roles (*){}", cache_indicator)
        } else {
            format!("Profiles & Roles{}", cache_indicator)
        };

        let table = Table::new(
            rows,
            [
                Constraint::Length(6),  // Type
                Constraint::Length(6),  // Status
                Constraint::Length(7),  // Default
                Constraint::Min(15),    // Account Name
                Constraint::Length(12), // Account ID
                Constraint::Min(15),    // Role Name
                Constraint::Min(15),    // Profile Name
                Constraint::Length(10), // Expiration
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(accounts_title)
                .border_style(accounts_block_style),
        )
        .row_highlight_style(
            Style::default()
                .bg(catppuccin_color(self.theme.colors.surface1))
                .add_modifier(Modifier::BOLD),
        );

        f.render_stateful_widget(table, chunks[1], &mut self.accounts_list_state);

        // Render scrollbar for accounts pane
        if !self.accounts.is_empty() {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state = ScrollbarState::new(self.accounts.len())
                .position(self.accounts_list_state.selected().unwrap_or(0));

            f.render_stateful_widget(
                scrollbar,
                chunks[1].inner(ratatui::layout::Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }

        // Sessions pane
        self.draw_sessions_pane(f, chunks[2]);

        // Help bar (2 lines for better readability)
        // Make Enter key description context-aware
        let enter_action = match self.active_pane {
            ActivePane::Sessions => "Enter:login/logout session",
            ActivePane::Accounts => "Enter:activate/deactivate profile",
        };

        let help_lines = vec![
            Line::from(vec![Span::raw(format!(
                "q:quit | ?:help | Tab:switch pane | ↑↓/jk:navigate | {}",
                enter_action
            ))]),
            Line::from(vec![
                Span::raw("Sessions: "),
                Span::styled("a", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(":add "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(":edit "),
                Span::styled("d", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(":delete "),
                Span::styled("f", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(":filter | Profiles: "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(":edit "),
                Span::styled("v", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(":view "),
                Span::styled("d", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(":default "),
                Span::styled("D", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(":delete "),
                Span::styled("c", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(":console "),
                Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(":refresh "),
                Span::styled("s", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(":SSM browser"),
            ]),
        ];
        let help_bar = Paragraph::new(help_lines)
            .style(Style::default().fg(catppuccin_color(self.theme.colors.subtext0)));
        f.render_widget(help_bar, chunks[3]);
    }

    fn draw_sessions_pane(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        let rows: Vec<Row> = self
            .sso_sessions
            .iter()
            .map(|session| {
                // Calculate expiration status first
                let (is_actually_active, expiration_status) = if session.is_active {
                    if let Some(expiration) = session.token_expiration {
                        let now = chrono::Utc::now();
                        let remaining_secs = (expiration - now).num_seconds();

                        if remaining_secs > 0 {
                            let hours = remaining_secs / 3600;
                            let mins = (remaining_secs % 3600) / 60;

                            let display = if hours > 0 {
                                format!("{}h {}m", hours, mins)
                            } else {
                                format!("{}m", mins)
                            };
                            (true, display)
                        } else {
                            (false, "EXPIRED".to_string())
                        }
                    } else {
                        (true, "".to_string())
                    }
                } else {
                    (false, "".to_string())
                };

                // Status indicator based on actual expiration state
                let status = if is_actually_active { "🟢" } else { "🔴" };

                // Add filter marker if this session is filtered
                let session_name_display =
                    if self.filtered_session.as_ref() == Some(&session.session_name) {
                        format!("{} [FILTERED]", session.session_name)
                    } else {
                        session.session_name.clone()
                    };

                Row::new(vec![
                    Cell::new(Text::from(status).alignment(Alignment::Center)),
                    Cell::new(Text::from(session_name_display).alignment(Alignment::Center)),
                    Cell::new(Text::from(session.start_url.clone()).alignment(Alignment::Center)),
                    Cell::new(Text::from(expiration_status).alignment(Alignment::Center)),
                ])
            })
            .collect();

        let header = Row::new(vec![
            Cell::new(Text::from("Status").alignment(Alignment::Center)),
            Cell::new(Text::from("Session Name").alignment(Alignment::Center)),
            Cell::new(Text::from("Start URL").alignment(Alignment::Center)),
            Cell::new(Text::from("Expires").alignment(Alignment::Center)),
        ])
        .style(
            Style::default()
                .fg(catppuccin_color(self.theme.colors.blue))
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

        // Highlight sessions pane if it's active
        let sessions_block_style = if self.active_pane == ActivePane::Sessions {
            Style::default().fg(catppuccin_color(self.theme.colors.mauve))
        } else {
            Style::default().fg(catppuccin_color(self.theme.colors.surface0))
        };

        // Add asterisk to title if this pane is active
        let sessions_title = if self.active_pane == ActivePane::Sessions {
            "SSO Sessions (*)"
        } else {
            "SSO Sessions"
        };

        let table = Table::new(
            rows,
            [
                Constraint::Length(6),  // Status
                Constraint::Min(20),    // Session Name
                Constraint::Min(30),    // Start URL
                Constraint::Length(10), // Expiration
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(sessions_title)
                .border_style(sessions_block_style),
        )
        .row_highlight_style(
            Style::default()
                .bg(catppuccin_color(self.theme.colors.surface1))
                .add_modifier(Modifier::BOLD),
        );

        f.render_stateful_widget(table, area, &mut self.sessions_list_state);

        // Render scrollbar for sessions pane
        if !self.sso_sessions.is_empty() {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state = ScrollbarState::new(self.sso_sessions.len())
                .position(self.sessions_list_state.selected().unwrap_or(0));

            f.render_stateful_widget(
                scrollbar,
                area.inner(ratatui::layout::Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }
    }

    fn draw_help_screen(&self, f: &mut Frame) {
        let help_text = vec![
            Line::from(Span::styled(
                "awsom - Help",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("Navigation:"),
            Line::from("  Tab         - Switch between Sessions and Profiles panes"),
            Line::from("  ↑, k        - Move selection up"),
            Line::from("  ↓, j        - Move selection down"),
            Line::from(""),
            Line::from("Sessions Pane:"),
            Line::from("  Enter       - Login/Logout selected SSO session"),
            Line::from("  a           - Add new SSO session"),
            Line::from("  e           - Edit selected SSO session"),
            Line::from("  d           - Delete selected SSO session"),
            Line::from(""),
            Line::from("Profiles Pane:"),
            Line::from("  Enter       - Start/stop session (activate/invalidate credentials)"),
            Line::from("  v           - View profile details"),
            Line::from("  e           - Edit profile (name, region, output) for selected role"),
            Line::from("  d           - Make selected profile the default"),
            Line::from("  c           - Open AWS Console in browser for selected profile"),
            Line::from("  r           - Refresh profile list"),
            Line::from(""),
            Line::from("General:"),
            Line::from("  q, Esc      - Quit application"),
            Line::from("  ?, F1       - Show this help screen"),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to return to main screen",
                Style::default().fg(Color::Yellow),
            )),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .style(Style::default().fg(Color::White));
        f.render_widget(help, f.area());
    }

    fn draw_view_profile_screen(&self, f: &mut Frame, details: Vec<(String, String)>) {
        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(
                "Profile Details",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        // Find max key length for alignment
        let max_key_len = details.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

        // Add details
        for (key, value) in details {
            let padded_key = format!("{:>width$}", key, width = max_key_len);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}: ", padded_key),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(value, Style::default().fg(Color::White)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press any key to return",
            Style::default().fg(Color::DarkGray),
        )));

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("View Profile"))
            .style(Style::default().fg(Color::White));
        f.render_widget(paragraph, f.area());
    }

    fn draw_loading_screen(&mut self, f: &mut Frame) {
        // Spinner frames for animation (Braille pattern spinner)
        const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let spinner_frame = SPINNER_FRAMES[self.tick_count as usize % SPINNER_FRAMES.len()];

        // Poll device_auth_info from watch receiver if available
        if let Some(ref rx) = self.device_auth_rx {
            self.device_auth_info = rx.borrow().clone();
        }

        let mut loading_text = vec![];

        // Check if we're showing device auth info
        if let Some(ref auth_info) = self.device_auth_info {
            loading_text.push(Line::from(Span::styled(
                "AWS SSO Login",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            loading_text.push(Line::from(""));

            // Use complete URL with code if available, otherwise show URL + code separately
            if let Some(ref complete_url) = auth_info.verification_uri_complete {
                // Show single URL with code embedded
                let instruction_text = if crate::env::is_headless_environment() {
                    "Copy and paste this URL (code is already included):"
                } else {
                    "Browser opened automatically. If not, copy this URL:"
                };

                loading_text.push(Line::from(Span::styled(
                    instruction_text,
                    Style::default().fg(Color::White),
                )));
                loading_text.push(Line::from(""));
                loading_text.push(Line::from(Span::styled(
                    complete_url,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )));
            } else {
                // Fallback: show URL and code separately
                let instruction_text = if crate::env::is_headless_environment() {
                    "Open this URL in a browser (on another machine if needed):"
                } else {
                    "Browser opened automatically. If not, visit:"
                };

                loading_text.push(Line::from(Span::styled(
                    instruction_text,
                    Style::default().fg(Color::White),
                )));
                loading_text.push(Line::from(""));
                loading_text.push(Line::from(Span::styled(
                    &auth_info.verification_uri,
                    Style::default().fg(Color::Green),
                )));
                loading_text.push(Line::from(""));
                loading_text.push(Line::from(Span::styled(
                    "And enter code:",
                    Style::default().fg(Color::White),
                )));
                loading_text.push(Line::from(""));
                loading_text.push(Line::from(Span::styled(
                    &auth_info.user_code,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            loading_text.push(Line::from(""));
            loading_text.push(Line::from(vec![
                Span::styled(
                    format!("{} ", spinner_frame),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    "Waiting for authorization...",
                    Style::default().fg(Color::Gray),
                ),
            ]));
            loading_text.push(Line::from(""));
            loading_text.push(Line::from(Span::styled(
                "Press 'q' or 'Esc' to cancel",
                Style::default().fg(Color::Yellow),
            )));
        } else {
            // Generic loading message with spinner
            loading_text.push(Line::from(""));
            loading_text.push(Line::from(vec![
                Span::styled(
                    format!("{} ", spinner_frame),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    "Loading...",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        let loading = Paragraph::new(loading_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(loading, f.area());
    }

    fn draw_error_screen(&self, f: &mut Frame, message: String) {
        let error_text = vec![
            Line::from(Span::styled(
                "Error",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(message),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to continue",
                Style::default().fg(Color::Yellow),
            )),
        ];

        let error = Paragraph::new(error_text)
            .block(Block::default().borders(Borders::ALL).title("Error"))
            .style(Style::default().fg(Color::White));
        f.render_widget(error, f.area());
    }

    fn draw_profile_input_screen(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(5), // Info
                Constraint::Length(3), // Input
                Constraint::Min(0),    // Spacer
                Constraint::Length(2), // Instructions
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("Save AWS Profile")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Info about the role
        let info_text = if let Some(ref role) = self.pending_role {
            vec![
                Line::from(format!(
                    "Account: {} ({})",
                    role.account_name, role.account_id
                )),
                Line::from(format!("Role: {}", role.role_name)),
                Line::from(""),
                Line::from("Enter a profile name (or press Enter to use default):"),
            ]
        } else {
            vec![Line::from("No role selected")]
        };

        let info = Paragraph::new(info_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(info, chunks[1]);

        // Input field with cursor at the correct position
        let input_with_cursor = if self.profile_input.is_empty() {
            "█".to_string()
        } else {
            // Split the string at cursor position and insert cursor character
            let (before, after) = self.profile_input.split_at(self.profile_input_cursor);
            format!("{}█{}", before, after)
        };
        let input = Paragraph::new(input_with_cursor.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Profile Name"));
        f.render_widget(input, chunks[2]);

        // Instructions
        let instructions = Paragraph::new(
            "Enter: Save | Esc: Cancel | ←→: Move cursor | Home/End: Jump | Type to edit",
        )
        .style(Style::default().fg(Color::Gray));
        f.render_widget(instructions, chunks[4]);
    }

    fn draw_sso_config_input_screen(&self, f: &mut Frame, step: SsoConfigStep) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(10), // Instructions
                Constraint::Length(3),  // Input
                Constraint::Min(0),     // Spacer
                Constraint::Length(2),  // Help
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("AWS SSO Configuration")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Instructions based on current step
        let (step_title, instructions, example) = match step {
            SsoConfigStep::StartUrl => (
                "Step 1 of 3: SSO Start URL",
                "Enter your AWS SSO start URL (IAM Identity Center portal URL)",
                "Example: https://my-org.awsapps.com/start",
            ),
            SsoConfigStep::Region => (
                "Step 2 of 3: SSO Region",
                "Enter the AWS Region where SSO is configured",
                "Example: us-east-1",
            ),
            SsoConfigStep::SessionName => (
                "Step 3 of 3: Session Name",
                "Enter a name for this SSO session (optional)",
                "Default: default-sso",
            ),
        };

        let info_text = vec![
            Line::from(Span::styled(
                step_title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(instructions),
            Line::from(""),
            Line::from(Span::styled(example, Style::default().fg(Color::Gray))),
            Line::from(""),
            Line::from("The configuration will be saved to ~/.aws/config"),
            Line::from("as a [sso-session] section."),
        ];

        let info = Paragraph::new(info_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(info, chunks[1]);

        // Input field with cursor
        let (current_input, field_label) = match step {
            SsoConfigStep::StartUrl => (&self.sso_start_url_input, "SSO Start URL"),
            SsoConfigStep::Region => (&self.sso_region_input, "SSO Region"),
            SsoConfigStep::SessionName => (&self.sso_session_name_input, "Session Name"),
        };

        let input_with_cursor = if current_input.is_empty() {
            "█".to_string()
        } else {
            let (before, after) = current_input.split_at(self.sso_input_cursor);
            format!("{}█{}", before, after)
        };

        let input = Paragraph::new(input_with_cursor.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(field_label));
        f.render_widget(input, chunks[2]);

        // Help
        let help = Paragraph::new("Enter: Next | Esc: Cancel | ←→: Move cursor | Type to edit")
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[4]);
    }

    fn draw_defaults_config_input_screen(&self, f: &mut Frame, step: DefaultsConfigStep) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(10), // Instructions
                Constraint::Length(3),  // Input
                Constraint::Min(0),     // Spacer
                Constraint::Length(2),  // Help
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("Configure Default Profile Settings")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Instructions
        let (step_title, instructions, example) = match step {
            DefaultsConfigStep::Region => (
                "Step 1 of 2: Default Region",
                "Enter the default AWS region for new profiles",
                "Example: us-east-1, eu-west-1, ap-southeast-2",
            ),
            DefaultsConfigStep::Output => (
                "Step 2 of 2: Default Output Format",
                "Enter the default output format for AWS CLI",
                "Options: json, text, table, yaml",
            ),
        };

        let info_text = vec![
            Line::from(Span::styled(
                step_title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(instructions),
            Line::from(""),
            Line::from(Span::styled(example, Style::default().fg(Color::Gray))),
            Line::from(""),
            Line::from("These settings will be saved to ~/.aws/config as:"),
            Line::from("[profile awsom-defaults]"),
            Line::from("This allows awsom to provide defaults without interfering with"),
            Line::from("your [default] profile."),
        ];

        let info = Paragraph::new(info_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(info, chunks[1]);

        // Input field
        let (current_input, field_label) = match step {
            DefaultsConfigStep::Region => (&self.default_region_input, "Default Region"),
            DefaultsConfigStep::Output => (&self.default_output_input, "Default Output Format"),
        };

        let input_with_cursor = if current_input.is_empty() {
            "█".to_string()
        } else {
            let (before, after) = current_input.split_at(self.default_input_cursor);
            format!("{}█{}", before, after)
        };

        let input = Paragraph::new(input_with_cursor.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(field_label));
        f.render_widget(input, chunks[2]);

        // Help
        let help = Paragraph::new("Enter: Next | Esc: Cancel | ←→: Move cursor | Type to edit")
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[4]);
    }

    fn draw_static_credential_input_screen(&self, f: &mut Frame, step: StaticCredentialStep) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(12), // Instructions
                Constraint::Length(3),  // Input
                Constraint::Min(0),     // Spacer
                Constraint::Length(2),  // Help
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("Add Static AWS Credentials")
            .style(
                Style::default()
                    .fg(catppuccin_color(self.theme.colors.mauve))
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Instructions based on current step
        let (step_title, instructions, example) = match step {
            StaticCredentialStep::ProfileName => (
                "Step 1 of 4: Profile Name",
                "Enter a name for this credential profile",
                "Example: my-dev-profile",
            ),
            StaticCredentialStep::AccessKeyId => (
                "Step 2 of 4: AWS Access Key ID",
                "Enter your AWS Access Key ID",
                "Example: AKIAIOSFODNN7EXAMPLE",
            ),
            StaticCredentialStep::SecretAccessKey => (
                "Step 3 of 4: AWS Secret Access Key",
                "Enter your AWS Secret Access Key",
                "Example: wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            ),
            StaticCredentialStep::SessionToken => (
                "Step 4 of 4: Session Token (Optional)",
                "Enter session token for temporary credentials (or leave empty)",
                "Leave empty for long-term credentials",
            ),
        };

        let mut info_text = vec![
            Line::from(Span::styled(
                step_title,
                Style::default()
                    .fg(catppuccin_color(self.theme.colors.yellow))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(instructions),
            Line::from(""),
            Line::from(Span::styled(
                example,
                Style::default().fg(catppuccin_color(self.theme.colors.overlay0)),
            )),
            Line::from(""),
        ];

        // Add warning for secret fields
        if matches!(
            step,
            StaticCredentialStep::SecretAccessKey | StaticCredentialStep::SessionToken
        ) {
            info_text.push(Line::from(Span::styled(
                "⚠  WARNING: This will be stored in plaintext in ~/.aws/credentials",
                Style::default().fg(catppuccin_color(self.theme.colors.red)),
            )));
        } else {
            info_text.push(Line::from(
                "The credentials will be saved to ~/.aws/credentials",
            ));
        }

        let info = Paragraph::new(info_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(info, chunks[1]);

        // Input field with cursor (mask secret key and session token)
        let (current_input, field_label, mask_input) = match step {
            StaticCredentialStep::ProfileName => {
                (&self.static_profile_name_input, "Profile Name", false)
            }
            StaticCredentialStep::AccessKeyId => {
                (&self.static_access_key_input, "Access Key ID", false)
            }
            StaticCredentialStep::SecretAccessKey => {
                (&self.static_secret_key_input, "Secret Access Key", true)
            }
            StaticCredentialStep::SessionToken => (
                &self.static_session_token_input,
                "Session Token (Optional)",
                true,
            ),
        };

        let display_text = if mask_input && !current_input.is_empty() {
            "*".repeat(current_input.len())
        } else {
            current_input.clone()
        };

        let input_with_cursor = if display_text.is_empty() {
            "█".to_string()
        } else {
            let cursor_pos = self.static_input_cursor.min(display_text.len());
            let (before, after) = display_text.split_at(cursor_pos);
            format!("{}█{}", before, after)
        };

        let input = Paragraph::new(input_with_cursor.as_str())
            .style(Style::default().fg(catppuccin_color(self.theme.colors.yellow)))
            .block(Block::default().borders(Borders::ALL).title(field_label));
        f.render_widget(input, chunks[2]);

        // Help
        let help = Paragraph::new("Enter: Next | Esc: Cancel | ←→: Move cursor | Type to edit")
            .style(Style::default().fg(catppuccin_color(self.theme.colors.subtext0)));
        f.render_widget(help, chunks[4]);
    }

    fn draw_new_profile_config_input_screen(&self, f: &mut Frame, step: NewProfileConfigStep) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(12), // Instructions
                Constraint::Length(3),  // Input
                Constraint::Min(0),     // Spacer
                Constraint::Length(2),  // Help
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("Configure New AWS Profile")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Instructions
        let (step_title, instructions, example) = match step {
            NewProfileConfigStep::ProfileName => (
                "Step 1 of 3: Profile Name",
                "Enter a name for this AWS profile",
                "Example: my-prod-account, dev-readonly",
            ),
            NewProfileConfigStep::Region => (
                "Step 2 of 3: Region",
                "Enter the AWS region for this profile",
                "Example: us-east-1, eu-west-1, ap-southeast-2",
            ),
            NewProfileConfigStep::Output => (
                "Step 3 of 3: Output Format",
                "Enter the output format for AWS CLI",
                "Options: json, text, table, yaml",
            ),
        };

        let info_text = vec![
            Line::from(Span::styled(
                step_title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(instructions),
            Line::from(""),
            Line::from(Span::styled(example, Style::default().fg(Color::Gray))),
            Line::from(""),
            Line::from("The profile will be saved to ~/.aws/config and"),
            Line::from("credentials will be written to ~/.aws/credentials"),
            Line::from(""),
            Line::from(Span::styled(
                "After configuration, credentials will be fetched automatically.",
                Style::default().fg(Color::Green),
            )),
        ];

        let info = Paragraph::new(info_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(info, chunks[1]);

        // Input field
        let (current_input, field_label, cursor_pos) = match step {
            NewProfileConfigStep::ProfileName => (
                &self.new_profile_name_input,
                "Profile Name",
                self.new_profile_input_cursor,
            ),
            NewProfileConfigStep::Region => (
                &self.new_profile_region_input,
                "Region",
                self.new_profile_input_cursor,
            ),
            NewProfileConfigStep::Output => (
                &self.new_profile_output_input,
                "Output Format",
                self.new_profile_input_cursor,
            ),
        };

        let input_with_cursor = if current_input.is_empty() {
            "█".to_string()
        } else {
            let (before, after) = current_input.split_at(cursor_pos);
            format!("{}█{}", before, after)
        };

        let input = Paragraph::new(input_with_cursor.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(field_label));
        f.render_widget(input, chunks[2]);

        // Help
        let help = Paragraph::new("Enter: Next | Esc: Cancel | ←→: Move cursor | Type to edit")
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[4]);
    }

    // ==================== SSM Browser ====================

    /// Open SSM browser for the selected profile
    async fn open_ssm_browser(&mut self) -> Result<()> {
        // Get currently selected profile
        let selected_idx = match self.accounts_list_state.selected() {
            Some(idx) => idx,
            None => {
                self.status_message = Some("No profile selected".to_string());
                return Ok(());
            }
        };

        let profile = match self.accounts.get(selected_idx) {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        // Only SSO profiles with credentials can use SSM
        let (profile_name, is_active) = match &profile {
            ProfileEntry::Sso(status) => {
                let name = status.profile_name.clone().unwrap_or_else(|| {
                    format!(
                        "{}/{}",
                        status.account_role.account_name, status.account_role.role_name
                    )
                });
                (name, status.is_active)
            }
            ProfileEntry::Static { profile_name, .. } => (profile_name.clone(), true),
            ProfileEntry::Incomplete { profile_name, .. } => {
                self.status_message = Some(format!(
                    "Profile '{}' has no credentials. Configure it first.",
                    profile_name
                ));
                return Ok(());
            }
        };

        if !is_active {
            self.status_message = Some(format!(
                "Profile '{}' has no active credentials. Press Enter to get credentials first.",
                profile_name
            ));
            return Ok(());
        }

        // Switch to SSM browser state
        self.state = AppState::SsmBrowser;
        self.ssm_loading = true;
        self.ssm_instances.clear();
        self.ssm_filter.clear();
        self.ssm_list_state.select(None);
        self.status_message = Some(format!("Loading instances for '{}'...", profile_name));

        // Load instances in background
        self.load_ssm_instances(&profile_name).await;

        Ok(())
    }

    /// Load SSM instances for the given profile
    async fn load_ssm_instances(&mut self, profile_name: &str) {
        // Read credentials from the profile
        let creds = match crate::aws_config::get_profile_credentials(profile_name) {
            Ok(Some(c)) => c,
            Ok(None) => {
                self.status_message = Some(format!("No credentials found for '{}'", profile_name));
                self.ssm_loading = false;
                return;
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to read credentials: {}", e));
                self.ssm_loading = false;
                return;
            }
        };

        // Get the region for this profile (default to us-east-1)
        let region = crate::aws_config::get_profile_region(profile_name)
            .unwrap_or_else(|_| Some("us-east-1".to_string()))
            .unwrap_or_else(|| "us-east-1".to_string());

        // Create SSM client with credentials
        let client = match crate::ssm::SsmSdkClient::with_credentials(
            &region,
            &creds.access_key_id,
            &creds.secret_access_key,
            creds.session_token.as_deref().unwrap_or(""),
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                self.status_message = Some(format!("Failed to create SSM client: {}", e));
                self.ssm_loading = false;
                return;
            }
        };

        // List instances
        match client.list_instances().await {
            Ok(instances) => {
                let count = instances.len();
                self.ssm_instances = instances;
                self.ssm_loading = false;
                if count > 0 {
                    self.ssm_list_state.select(Some(0));
                    self.status_message = Some(format!("Found {} instance(s)", count));
                } else {
                    self.status_message = Some("No EC2 instances found".to_string());
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to list instances: {}", e));
                self.ssm_loading = false;
            }
        }
    }

    /// Handle key events in SSM browser
    async fn handle_ssm_browser_key(&mut self, key: KeyCode) -> Result<()> {
        // If in search mode, handle search input
        if self.ssm_search_mode {
            match key {
                KeyCode::Esc => {
                    // Exit search mode
                    self.ssm_search_mode = false;
                }
                KeyCode::Enter => {
                    // Exit search mode and keep filter
                    self.ssm_search_mode = false;
                }
                KeyCode::Backspace => {
                    // Delete last character
                    self.ssm_filter.pop();
                }
                KeyCode::Char(c) => {
                    // Add character to filter
                    self.ssm_filter.push(c);
                }
                _ => {}
            }
            return Ok(());
        }

        // Normal mode key handling
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Return to main screen
                self.state = AppState::Main;
                self.ssm_instances.clear();
                self.ssm_filter.clear();
                self.ssm_search_mode = false;
            }
            KeyCode::Char('/') => {
                // Enter search mode
                self.ssm_search_mode = true;
                self.ssm_filter.clear();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Next instance
                self.next_ssm_instance();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                // Previous instance
                self.previous_ssm_instance();
            }
            KeyCode::Enter => {
                // Start SSM session
                self.start_ssm_session()?;
            }
            KeyCode::Char('y') => {
                // Copy command to clipboard (just show it for now)
                self.copy_ssm_command();
            }
            KeyCode::Char('r') => {
                // Refresh instance list
                if let Some(profile) = self.get_selected_profile_name() {
                    self.ssm_loading = true;
                    self.status_message = Some("Refreshing instances...".to_string());
                    self.load_ssm_instances(&profile).await;
                }
            }
            KeyCode::Char('o') => {
                // Toggle showing offline instances
                self.ssm_show_offline = !self.ssm_show_offline;
                let status = if self.ssm_show_offline {
                    "Showing all instances (online and offline)"
                } else {
                    "Showing only online instances"
                };
                self.status_message = Some(status.to_string());
            }
            KeyCode::Char('s') => {
                // Cycle through sort orders
                self.ssm_sort_order = self.ssm_sort_order.next();
                self.status_message = Some(format!("Sort by: {}", self.ssm_sort_order.as_str()));
            }
            KeyCode::Char('v') => {
                // View instance tags
                if let Some(instance) = self.get_selected_ssm_instance() {
                    // Convert tags HashMap to sorted Vec for display
                    let mut tags: Vec<(String, String)> = instance
                        .tags
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    tags.sort_by(|a, b| a.0.cmp(&b.0));
                    self.state = AppState::ViewInstanceTags { tags };
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn next_ssm_instance(&mut self) {
        let filtered = self.filtered_ssm_instances();
        if filtered.is_empty() {
            return;
        }
        let i = match self.ssm_list_state.selected() {
            Some(i) => {
                if i >= filtered.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.ssm_list_state.select(Some(i));
    }

    fn previous_ssm_instance(&mut self) {
        let filtered = self.filtered_ssm_instances();
        if filtered.is_empty() {
            return;
        }
        let i = match self.ssm_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    filtered.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.ssm_list_state.select(Some(i));
    }

    fn filtered_ssm_instances(&self) -> Vec<&crate::ssm::SsmInstance> {
        let filter = self.ssm_filter.to_lowercase();
        let mut instances: Vec<&crate::ssm::SsmInstance> = self
            .ssm_instances
            .iter()
            .filter(|i| {
                // Always exclude terminated instances
                if i.state == "terminated" {
                    return false;
                }

                // Filter by online status if toggle is off
                if !self.ssm_show_offline && !i.ssm_status.is_connectable() {
                    return false;
                }

                // Apply search filter if present
                if !filter.is_empty() {
                    return i.name.to_lowercase().contains(&filter)
                        || i.instance_id.to_lowercase().contains(&filter)
                        || i.private_ip.as_ref().is_some_and(|ip| ip.contains(&filter));
                }

                true
            })
            .collect();

        // Apply sorting based on current sort order
        match self.ssm_sort_order {
            SsmSortOrder::None => {
                // No sorting, keep API order
            }
            SsmSortOrder::Name => {
                instances.sort_by(|a, b| {
                    a.name
                        .to_lowercase()
                        .cmp(&b.name.to_lowercase())
                        .then_with(|| a.instance_id.cmp(&b.instance_id))
                });
            }
            SsmSortOrder::InstanceId => {
                instances.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
            }
            SsmSortOrder::PrivateIp => {
                instances.sort_by(|a, b| match (&a.private_ip, &b.private_ip) {
                    (Some(a_ip), Some(b_ip)) => a_ip.cmp(b_ip),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.instance_id.cmp(&b.instance_id),
                });
            }
        }

        instances
    }

    fn get_selected_ssm_instance(&self) -> Option<&crate::ssm::SsmInstance> {
        let filtered = self.filtered_ssm_instances();
        self.ssm_list_state
            .selected()
            .and_then(|idx| filtered.get(idx).copied())
    }

    fn get_selected_profile_name(&self) -> Option<String> {
        self.accounts_list_state.selected().and_then(|idx| {
            self.accounts.get(idx).and_then(|p| match p {
                ProfileEntry::Sso(status) => status.profile_name.clone(),
                ProfileEntry::Static { profile_name, .. } => Some(profile_name.clone()),
                ProfileEntry::Incomplete { .. } => None,
            })
        })
    }

    fn start_ssm_session(&mut self) -> Result<()> {
        let instance = match self.get_selected_ssm_instance() {
            Some(i) => i.clone(),
            None => {
                self.status_message = Some("No instance selected".to_string());
                return Ok(());
            }
        };

        if !instance.ssm_status.is_connectable() {
            self.status_message = Some(format!(
                "Instance {} is not SSM-connectable (status: {})",
                instance.instance_id,
                instance.ssm_status.as_str()
            ));
            return Ok(());
        }

        // Get profile name and region
        let profile_name = match self.get_selected_profile_name() {
            Some(p) => p,
            None => {
                self.status_message = Some("No profile selected".to_string());
                return Ok(());
            }
        };

        let region = crate::aws_config::get_profile_region(&profile_name)
            .ok()
            .flatten()
            .unwrap_or_else(|| "us-east-1".to_string());

        // Suspend TUI before running SSM session
        disable_raw_mode()?;
        execute!(std::io::stdout(), LeaveAlternateScreen)?;

        // Install panic hook to restore terminal if something goes wrong
        let original_hook = std::panic::take_hook();
        let hook_clone = std::sync::Arc::new(original_hook);
        let hook_for_panic = hook_clone.clone();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
            hook_for_panic(panic_info);
        }));

        // Run SSM session in current terminal
        println!(
            "Starting SSM session to {} ({})...\n",
            instance.name, instance.instance_id
        );

        let mut child = std::process::Command::new("aws")
            .arg("ssm")
            .arg("start-session")
            .arg("--target")
            .arg(&instance.instance_id)
            .arg("--region")
            .arg(&region)
            .env("AWS_PROFILE", &profile_name)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn();

        let status = match child {
            Ok(ref mut process) => process.wait(),
            Err(e) => Err(e),
        };

        // Restore original panic hook
        let _ = std::panic::take_hook();
        match std::sync::Arc::try_unwrap(hook_clone) {
            Ok(hook) => std::panic::set_hook(hook),
            Err(_) => {
                // If we can't unwrap, just use default panic behavior
                std::panic::set_hook(Box::new(|info| {
                    eprintln!("{}", info);
                }));
            }
        }

        // Resume TUI
        execute!(std::io::stdout(), EnterAlternateScreen)?;
        enable_raw_mode()?;

        // Set status message based on result
        match status {
            Ok(exit_status) => {
                if exit_status.success() {
                    self.status_message = Some(format!(
                        "Session to {} ({}) ended successfully",
                        instance.name, instance.instance_id
                    ));
                } else {
                    self.status_message = Some(format!(
                        "Session to {} ({}) exited with code: {}",
                        instance.name,
                        instance.instance_id,
                        exit_status.code().unwrap_or(-1)
                    ));
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to start session: {}", e));
            }
        }

        Ok(())
    }

    fn copy_ssm_command(&mut self) {
        let instance = match self.get_selected_ssm_instance() {
            Some(i) => i,
            None => {
                self.status_message = Some("No instance selected".to_string());
                return;
            }
        };

        let profile_name = match self.get_selected_profile_name() {
            Some(p) => p,
            None => {
                self.status_message = Some("No profile selected".to_string());
                return;
            }
        };

        let region = crate::aws_config::get_profile_region(&profile_name)
            .ok()
            .flatten()
            .unwrap_or_else(|| "us-east-1".to_string());

        let cmd = format!(
            "AWS_PROFILE={} aws ssm start-session --target {} --region {}",
            profile_name, instance.instance_id, region
        );

        // Copy to clipboard
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(&cmd) {
                Ok(_) => {
                    self.status_message = Some(format!("✓ Copied to clipboard: {}", cmd));
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to copy: {} (Command: {})", e, cmd));
                }
            },
            Err(e) => {
                self.status_message =
                    Some(format!("Clipboard unavailable: {} (Command: {})", e, cmd));
            }
        }
    }

    fn draw_ssm_browser(&mut self, f: &mut Frame) {
        use ratatui::widgets::Row;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Instance list
                Constraint::Length(3), // Help
            ])
            .split(f.area());

        // Header
        let header_text = if self.ssm_search_mode {
            // Show search prompt when in search mode
            format!("Search: {}_", self.ssm_filter)
        } else if self.ssm_loading {
            let spinner_frames: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let spinner = spinner_frames[self.tick_count as usize % spinner_frames.len()];
            format!("{} SSM Browser - Loading instances...", spinner)
        } else {
            // Count online instances and total (excluding terminated)
            let online_count = self
                .ssm_instances
                .iter()
                .filter(|i| i.state != "terminated" && i.ssm_status.is_connectable())
                .count();
            let total_count = self
                .ssm_instances
                .iter()
                .filter(|i| i.state != "terminated")
                .count();
            format!(
                "SSM Browser - {} online / {} total",
                online_count, total_count
            )
        };
        let header = Paragraph::new(header_text)
            .style(
                Style::default()
                    .fg(catppuccin_color(self.theme.colors.blue))
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Instance table - collect owned data to avoid borrow issues
        let green = catppuccin_color(self.theme.colors.green);
        let red = catppuccin_color(self.theme.colors.red);
        let yellow = catppuccin_color(self.theme.colors.yellow);

        // Collect all data into owned values to release the borrow on self
        let instance_data: Vec<(String, String, String, String, String, bool, bool)> = self
            .filtered_ssm_instances()
            .iter()
            .map(|i| {
                (
                    i.ssm_status.as_str().to_string(),
                    i.name.clone(),
                    i.instance_id.clone(),
                    i.state.clone(),
                    i.private_ip.clone().unwrap_or_default(),
                    i.ssm_status.is_connectable(),
                    i.state == "running",
                )
            })
            .collect();

        let rows: Vec<Row> = instance_data
            .iter()
            .map(
                |(status, name, instance_id, state, private_ip, is_connectable, is_running)| {
                    let status_style = if *is_connectable {
                        Style::default().fg(green)
                    } else {
                        Style::default().fg(red)
                    };

                    let state_style = if *is_running {
                        Style::default().fg(green)
                    } else {
                        Style::default().fg(yellow)
                    };

                    Row::new(vec![
                        Cell::from(status.as_str()).style(status_style),
                        Cell::from(name.clone()),
                        Cell::from(instance_id.clone()),
                        Cell::from(state.clone()).style(state_style),
                        Cell::from(private_ip.clone()),
                    ])
                },
            )
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(8),  // SSM Status
                Constraint::Min(20),    // Name
                Constraint::Length(20), // Instance ID
                Constraint::Length(10), // State
                Constraint::Length(15), // Private IP
            ],
        )
        .header(
            Row::new(vec!["SSM", "Name", "Instance ID", "State", "Private IP"])
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("EC2 Instances"),
        )
        .row_highlight_style(
            Style::default()
                .bg(catppuccin_color(self.theme.colors.surface0))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

        f.render_stateful_widget(table, chunks[1], &mut self.ssm_list_state);

        // Help bar
        let help = Paragraph::new(
            "↑↓/jk: Navigate | Enter: Start session | v: View tags | y: Copy command | /: Search | s: Sort | o: Toggle offline | r: Refresh | q/Esc: Back",
        )
        .style(Style::default().fg(catppuccin_color(self.theme.colors.subtext0)));
        f.render_widget(help, chunks[2]);
    }

    fn draw_view_instance_tags_screen(&self, f: &mut Frame, tags: Vec<(String, String)>) {
        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(
                "Instance Tags",
                Style::default()
                    .fg(catppuccin_color(self.theme.colors.blue))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        if tags.is_empty() {
            lines.push(Line::from(Span::styled(
                "No tags found",
                Style::default().fg(catppuccin_color(self.theme.colors.subtext0)),
            )));
        } else {
            // Find max key length for alignment
            let max_key_len = tags.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

            // Add tags
            for (key, value) in tags {
                let padded_key = format!("{:>width$}", key, width = max_key_len);
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{}: ", padded_key),
                        Style::default()
                            .fg(catppuccin_color(self.theme.colors.yellow))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        value,
                        Style::default().fg(catppuccin_color(self.theme.colors.text)),
                    ),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press any key to return",
            Style::default().fg(catppuccin_color(self.theme.colors.subtext0)),
        )));

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("View Instance Tags"),
            )
            .style(Style::default().fg(catppuccin_color(self.theme.colors.text)));
        f.render_widget(paragraph, f.area());
    }
}
