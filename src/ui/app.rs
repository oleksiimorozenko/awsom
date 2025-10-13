// Main TUI application
use crate::auth::{AuthManager, DeviceAuthorizationInfo};
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

/// Convert Catppuccin color to Ratatui Color
fn catppuccin_color(color: catppuccin::Color) -> Color {
    Color::Rgb(color.rgb.r, color.rgb.g, color.rgb.b)
}

/// Wrapper for AccountRole with active status
#[derive(Debug, Clone)]
struct AccountRoleWithStatus {
    account_role: AccountRole,
    is_active: bool,
    expiration: Option<chrono::DateTime<chrono::Utc>>,
    is_default: bool,
}

/// SSO Session with its status
#[derive(Debug, Clone)]
struct SsoSessionInfo {
    session_name: String,
    start_url: String,
    region: String,
    is_active: bool,
    token_expiration: Option<chrono::DateTime<chrono::Utc>>,
    instance: SsoInstance,
    token: Option<SsoToken>,
}

/// Active pane in two-pane layout
#[derive(Debug, Clone, PartialEq)]
enum ActivePane {
    Sessions,
    Accounts,
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
    /// List of accounts and roles with their active status (filtered by selected session)
    accounts: Vec<AccountRoleWithStatus>,
    /// Accounts table selection state
    accounts_list_state: TableState,
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
    /// Device authorization info during login
    device_auth_info: Option<DeviceAuthorizationInfo>,
    /// Last Ctrl+C press time for double-press detection
    last_ctrl_c_time: Option<std::time::Instant>,
    /// SSO configuration input buffers
    sso_start_url_input: String,
    sso_region_input: String,
    sso_session_name_input: String,
    sso_input_cursor: usize,
    /// Default configuration input buffers
    default_region_input: String,
    default_output_input: String,
    default_input_cursor: usize,
    /// New profile configuration input buffers
    new_profile_name_input: String,
    new_profile_region_input: String,
    new_profile_output_input: String,
    new_profile_input_cursor: usize,
    /// Last automatic refresh time
    last_auto_refresh: Option<std::time::Instant>,
    /// Catppuccin theme flavor
    theme: Flavor,
}

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    /// Main screen showing account/role list
    Main,
    /// Help screen
    Help,
    /// Loading state
    Loading,
    /// Error state
    Error(String),
    /// Profile name input
    ProfileInput,
    /// SSO configuration input
    SsoConfigInput { step: SsoConfigStep },
    /// Default profile configuration input
    DefaultsConfigInput { step: DefaultsConfigStep },
    /// New profile configuration input (with region and output)
    NewProfileConfigInput { step: NewProfileConfigStep },
}

#[derive(Debug, Clone, PartialEq)]
enum SsoConfigStep {
    StartUrl,
    Region,
    SessionName,
}

#[derive(Debug, Clone, PartialEq)]
enum DefaultsConfigStep {
    Region,
    Output,
}

#[derive(Debug, Clone, PartialEq)]
enum NewProfileConfigStep {
    ProfileName,
    Region,
    Output,
}

impl App {
    pub fn new() -> Result<Self> {
        let auth_manager = AuthManager::new()?;
        let credential_manager = CredentialManager::new()?;

        Ok(Self {
            should_quit: false,
            state: AppState::Main,
            active_pane: ActivePane::Sessions,
            sso_sessions: Vec::new(),
            sessions_list_state: TableState::default(),
            accounts: Vec::new(),
            accounts_list_state: TableState::default(),
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
            last_ctrl_c_time: None,
            sso_start_url_input: String::new(),
            sso_region_input: String::new(),
            sso_session_name_input: "default-sso".to_string(),
            sso_input_cursor: 0,
            default_region_input: "us-east-1".to_string(),
            default_output_input: "json".to_string(),
            default_input_cursor: 0,
            new_profile_name_input: String::new(),
            new_profile_region_input: String::new(),
            new_profile_output_input: String::new(),
            new_profile_input_cursor: 0,
            last_auto_refresh: None,
            theme: catppuccin::PALETTE.mocha,
        })
    }

    /// Get the currently selected SSO session
    fn get_selected_session(&self) -> Option<&SsoSessionInfo> {
        self.sessions_list_state
            .selected()
            .and_then(|idx| self.sso_sessions.get(idx))
    }

    /// Get the currently selected SSO session's token
    fn get_current_token(&self) -> Option<&SsoToken> {
        self.get_selected_session()
            .and_then(|session| session.token.as_ref())
    }

    /// Get the currently selected SSO session's instance
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

        // Load all SSO sessions
        self.load_all_sso_sessions().await;

        // Load accounts for selected session if active
        if self.sso_token.is_some() {
            let _ = self.load_accounts().await;
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

        loop {
            terminal.draw(|f| self.ui(f)).map_err(SsoError::Io)?;

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

            if should_auto_refresh
                && self.state == AppState::Main
                && self.sso_token.is_some()
                && !self.accounts.is_empty()
            {
                tracing::debug!("Auto-refreshing account list (1 minute interval)");
                self.last_auto_refresh = Some(now);
                if let Err(e) = self.load_accounts().await {
                    tracing::warn!("Auto-refresh failed: {}", e);
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

    async fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        match self.state {
            AppState::Main => self.handle_main_key(key).await?,
            AppState::Help => {
                // Any key exits help screen
                self.state = AppState::Main;
            }
            AppState::Loading => {
                // In loading state, most keys are ignored (except Ctrl+C handled separately)
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
                // Refresh account list
                if self.sso_token.is_some() {
                    self.load_accounts().await?;
                    // Reset auto-refresh timer after manual refresh
                    self.last_auto_refresh = Some(std::time::Instant::now());
                } else {
                    self.status_message = Some(
                        "Not logged in. Switch to Sessions pane (Tab) and press Enter to login."
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
            KeyCode::Char('a') => {
                if self.active_pane == ActivePane::Sessions {
                    self.add_sso_session().await?;
                }
            }
            KeyCode::Char('e') => {
                if self.active_pane == ActivePane::Sessions {
                    self.edit_sso_session().await?;
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
            KeyCode::Char('p') => {
                if self.active_pane == ActivePane::Accounts {
                    // Edit profile name
                    self.edit_profile_name().await?;
                }
            }
            KeyCode::Char('c') => {
                if self.active_pane == ActivePane::Accounts {
                    // Open AWS Console in browser
                    self.open_console().await?;
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
            let instance_clone = instance.clone();

            // Perform login with callback
            match self
                .auth_manager
                .login_with_callback(&instance, false, |auth_info| {
                    self.device_auth_info = Some(auth_info.clone());

                    // Open browser
                    let url_to_open = auth_info
                        .verification_uri_complete
                        .as_ref()
                        .unwrap_or(&auth_info.verification_uri);

                    if let Err(e) = webbrowser::open(url_to_open) {
                        tracing::warn!("Could not open browser automatically: {}", e);
                    }

                    Ok(())
                })
                .await
            {
                Ok(token) => {
                    self.device_auth_info = None;

                    // Update session in list
                    if let Some(session_mut) = self.sso_sessions.get_mut(index) {
                        session_mut.is_active = true;
                        session_mut.token = Some(token.clone());
                        session_mut.token_expiration = Some(token.expires_at);
                    }

                    // Update current session
                    self.sso_instance = Some(instance_clone);
                    self.sso_token = Some(token);
                    self.state = AppState::Main;
                    self.status_message = Some(format!("✓ Logged in to {}", session.session_name));

                    // Load accounts for this session
                    self.load_accounts().await?;
                }
                Err(e) => {
                    self.device_auth_info = None;
                    self.state = AppState::Main;
                    self.status_message = Some(format!("Login failed: {}", e));
                }
            }
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

            // If this was the current session, clear it
            if let Some(ref current_instance) = self.sso_instance {
                if current_instance.start_url == session.start_url {
                    self.sso_instance = None;
                    self.sso_token = None;
                    self.accounts.clear();
                    self.accounts_list_state.select(None);
                }
            }

            self.status_message = Some(format!("✓ Logged out from {}", session.session_name));
        }
        Ok(())
    }

    /// Add a new SSO session
    async fn add_sso_session(&mut self) -> Result<()> {
        self.status_message = Some("Add SSO session - not yet implemented".to_string());
        // TODO: Implement session add dialog
        // Will need to:
        // 1. Show input dialog for session_name, sso_start_url, sso_region
        // 2. Write to ~/.aws/config
        // 3. Reload sessions list
        Ok(())
    }

    /// Edit the selected SSO session
    async fn edit_sso_session(&mut self) -> Result<()> {
        if let Some(index) = self.sessions_list_state.selected() {
            if let Some(session) = self.sso_sessions.get(index) {
                self.status_message = Some(format!(
                    "Edit SSO session '{}' - not yet implemented",
                    session.session_name
                ));
                // TODO: Implement session edit dialog
                // Will need to:
                // 1. Show input dialog pre-filled with current values
                // 2. Update ~/.aws/config
                // 3. Reload sessions list
            }
        } else {
            self.status_message = Some("No session selected".to_string());
        }
        Ok(())
    }

    /// Delete the selected SSO session
    async fn delete_sso_session(&mut self) -> Result<()> {
        if let Some(index) = self.sessions_list_state.selected() {
            if let Some(session) = self.sso_sessions.get(index).cloned() {
                // First, logout if the session is active
                if session.is_active {
                    self.logout_session(index).await?;
                }

                // Remove from the sessions list
                self.sso_sessions.remove(index);

                // Update selection
                if self.sso_sessions.is_empty() {
                    self.sessions_list_state.select(None);
                } else if index >= self.sso_sessions.len() {
                    self.sessions_list_state
                        .select(Some(self.sso_sessions.len() - 1));
                }

                self.status_message = Some(format!(
                    "✓ Deleted session '{}' (from memory only, still in ~/.aws/config)",
                    session.session_name
                ));

                // TODO: Also remove from ~/.aws/config file
                // For now, we only remove from memory - the session will reappear on restart
            }
        } else {
            self.status_message = Some("No session selected".to_string());
        }
        Ok(())
    }

    /// Toggle role session: if active, delete it; if inactive, create it
    async fn toggle_role_session(&mut self) -> Result<()> {
        if let Some(index) = self.accounts_list_state.selected() {
            if let Some(account_with_status) = self.accounts.get(index).cloned() {
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
                    // Check if there's an existing profile name for this role
                    let existing_profile = crate::aws_config::get_existing_profile_name(&account)?;

                    if let Some(profile_name) = existing_profile {
                        // Profile exists, just activate it
                        self.state = AppState::Loading;
                        self.save_profile_credentials(&account, &profile_name)
                            .await?;
                    } else {
                        // First time creating profile for this role
                        // Check if [default] section exists
                        match crate::aws_config::read_default_config()? {
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
                                // No defaults, show defaults config dialog first
                                self.pending_role = Some(account);
                                self.state = AppState::DefaultsConfigInput {
                                    step: DefaultsConfigStep::Region,
                                };
                                self.status_message = Some(
                                    "No [default] section found. Let's configure default settings first!".to_string()
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
            if let Some(account_with_status) = self.accounts.get(index).cloned() {
                let account = account_with_status.account_role;

                // Check if there's an existing profile for this role
                if let Some(existing_profile) =
                    crate::aws_config::get_existing_profile_name(&account)?
                {
                    // Don't rename if already default
                    if existing_profile == "default" {
                        self.status_message = Some("Profile is already set as default".to_string());
                        return Ok(());
                    }

                    // Check if a default profile already exists
                    let has_default = crate::aws_config::list_profile_statuses()?
                        .iter()
                        .any(|s| s.profile_name == "default");

                    if has_default {
                        // Delete the existing default profile first
                        tracing::info!("Deleting existing default profile");
                        if let Err(e) = crate::aws_config::delete_profile("default") {
                            tracing::warn!("Failed to delete existing default profile: {}", e);
                        }
                    }

                    // Rename the profile to default
                    match crate::aws_config::rename_profile(&existing_profile, "default") {
                        Ok(()) => {
                            self.status_message =
                                Some(format!("✓ Set '{}' as default profile", existing_profile));
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
                } else {
                    self.status_message = Some("No active profile found for this role. Press Enter to create credentials first.".to_string());
                }
            }
        }
        Ok(())
    }

    /// Open profile name editor for selected role
    async fn edit_profile_name(&mut self) -> Result<()> {
        if let Some(index) = self.accounts_list_state.selected() {
            if let Some(account_with_status) = self.accounts.get(index).cloned() {
                let account = account_with_status.account_role;

                // Check if there's an existing profile name for this role
                let existing_profile = crate::aws_config::get_existing_profile_name(&account)?;

                let profile_name = if let Some(ref existing) = existing_profile {
                    existing.clone()
                } else {
                    // Generate default profile name
                    format!(
                        "{}_{}",
                        account.account_name.replace(" ", "-").to_lowercase(),
                        account.role_name.replace(" ", "-").to_lowercase()
                    )
                };

                self.profile_input = profile_name.clone();
                self.profile_input_cursor = profile_name.len();
                self.pending_role = Some(account);
                self.existing_profile_name = existing_profile;
                self.state = AppState::ProfileInput;
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

                        match crate::aws_config::write_sso_session(&session) {
                            Ok(()) => {
                                self.status_message = Some(format!(
                                    "✓ SSO configuration saved to ~/.aws/config as [sso-session {}]",
                                    session_name
                                ));
                                self.state = AppState::Main;

                                // Clear input buffers
                                self.sso_start_url_input.clear();
                                self.sso_region_input.clear();
                                self.sso_session_name_input = "default-sso".to_string();
                                self.sso_input_cursor = 0;

                                // Automatically trigger login
                                self.login().await?;
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
                        // Save default configuration
                        let config = crate::aws_config::DefaultConfig {
                            region: self.default_region_input.trim().to_string(),
                            output: self.default_output_input.trim().to_string(),
                        };

                        match crate::aws_config::write_default_config(&config) {
                            Ok(()) => {
                                self.status_message = Some(
                                    "✓ Default configuration saved to ~/.aws/config".to_string(),
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
                                self.default_region_input = "us-east-1".to_string();
                                self.default_output_input = "json".to_string();
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
                self.default_region_input = "us-east-1".to_string();
                self.default_output_input = "json".to_string();
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

    async fn save_profile_credentials(
        &mut self,
        account: &AccountRole,
        profile_name: &str,
    ) -> Result<()> {
        if let (Some(ref token), Some(ref instance)) = (&self.sso_token, &self.sso_instance) {
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
        };

        // Perform login with callback to capture device auth info
        let instance_clone = instance.clone();
        match self
            .auth_manager
            .login_with_callback(&instance, false, |auth_info| {
                // Store device auth info for display in loading screen
                self.device_auth_info = Some(auth_info.clone());

                // Open browser
                let url_to_open = auth_info
                    .verification_uri_complete
                    .as_ref()
                    .unwrap_or(&auth_info.verification_uri);

                if let Err(e) = webbrowser::open(url_to_open) {
                    tracing::warn!("Could not open browser automatically: {}", e);
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

    async fn logout(&mut self) -> Result<()> {
        if let Some(ref instance) = self.sso_instance {
            // Remove cached token
            if let Err(e) = self.auth_manager.remove_token(instance) {
                tracing::warn!("Failed to remove cached token: {}", e);
            }
        }

        // Clear session data
        self.sso_token = None;
        self.sso_instance = None;
        self.accounts.clear();
        self.accounts_list_state.select(None);
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
                let mut sso_session_infos = Vec::new();

                for session in sessions {
                    let instance = SsoInstance {
                        start_url: session.sso_start_url.clone(),
                        region: session.sso_region.clone(),
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

                // Select first session if available
                if !self.sso_sessions.is_empty() && self.sessions_list_state.selected().is_none() {
                    self.sessions_list_state.select(Some(0));
                    // Set current session to first one if it's active
                    if let Some(first_session) = self.sso_sessions.first() {
                        if first_session.is_active {
                            self.sso_instance = Some(first_session.instance.clone());
                            self.sso_token = first_session.token.clone();
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

    async fn load_accounts(&mut self) -> Result<()> {
        if let (Some(ref token), Some(ref instance)) = (&self.sso_token, &self.sso_instance) {
            self.state = AppState::Loading;
            self.status_message = Some("Loading accounts and roles...".to_string());

            match self
                .credential_manager
                .list_accounts(&instance.region, &token.access_token)
                .await
            {
                Ok(account_list) => {
                    // Now fetch roles for each account
                    let mut all_roles = Vec::new();
                    for (account_id, account_name) in account_list {
                        match self
                            .credential_manager
                            .list_account_roles(&instance.region, &token.access_token, &account_id)
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

                    // Load credential statuses from AWS config
                    let statuses = crate::aws_config::list_profile_statuses().unwrap_or_default();

                    // Build a map from (account_id, role_name) to (is_active, expiration, is_default)
                    #[allow(clippy::type_complexity)]
                    let mut profile_map: HashMap<
                        (String, String),
                        (bool, Option<chrono::DateTime<chrono::Utc>>, bool),
                    > = HashMap::new();

                    for status in statuses {
                        if status.has_credentials {
                            if let (Some(account_id), Some(role_name)) =
                                (status.account_id, status.role_name)
                            {
                                // Check if this is the default profile
                                let is_default = status.profile_name == "default";
                                // Match by account ID and role name from metadata
                                profile_map.insert(
                                    (account_id, role_name),
                                    (true, status.expiration, is_default),
                                );
                            }
                        }
                    }

                    // Wrap roles with status
                    let mut accounts_with_status: Vec<AccountRoleWithStatus> = all_roles
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

                            AccountRoleWithStatus {
                                account_role,
                                is_active,
                                expiration,
                                is_default,
                            }
                        })
                        .collect();

                    // Sort by account name, then by role name
                    accounts_with_status.sort_by(|a, b| {
                        a.account_role
                            .account_name
                            .cmp(&b.account_role.account_name)
                            .then_with(|| a.account_role.role_name.cmp(&b.account_role.role_name))
                    });

                    self.accounts = accounts_with_status;
                    self.state = AppState::Main;
                    self.status_message = Some(format!(
                        "Loaded {} account/role combinations",
                        self.accounts.len()
                    ));

                    // Select first item if none selected
                    if self.accounts_list_state.selected().is_none() && !self.accounts.is_empty() {
                        self.accounts_list_state.select(Some(0));
                    }
                }
                Err(e) => {
                    self.state = AppState::Error(format!("Failed to load accounts: {}", e));
                }
            }
        }
        Ok(())
    }

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

    /// Open AWS Console in browser for selected role
    async fn open_console(&mut self) -> Result<()> {
        if let Some(index) = self.accounts_list_state.selected() {
            if let Some(account_with_status) = self.accounts.get(index).cloned() {
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
        }
    }

    fn draw_main_screen(&mut self, f: &mut Frame) {
        // Calculate dynamic sessions pane height
        // Min 5 lines (1 border top + 1 header + 1 header margin + 1 content + 1 border bottom)
        // Max 10 lines to avoid taking too much space
        let sessions_count = self.sso_sessions.len();
        let sessions_height = if sessions_count == 0 {
            5 // Minimum height for empty pane
        } else {
            // 3 for borders + header, plus 1 line per session, max 10 total
            std::cmp::min(sessions_count + 3, 10)
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                      // Header
                Constraint::Min(10),                        // Accounts pane (flexible)
                Constraint::Length(sessions_height as u16), // Sessions pane (dynamic)
                Constraint::Length(3),                      // Status
                Constraint::Length(1),                      // Help bar
            ])
            .split(f.area());

        // Header
        let header = Paragraph::new("awsom - AWS Organization Manager")
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
            .map(|account_with_status| {
                let account = &account_with_status.account_role;

                // Status indicator
                let status = if account_with_status.is_active {
                    "🟢"
                } else {
                    "🔴"
                };

                // Default marker
                let default_mark = if account_with_status.is_default {
                    "✓"
                } else {
                    ""
                };

                // Expiration status
                let expiration_status = if account_with_status.is_active {
                    if let Some(expiration) = account_with_status.expiration {
                        let now = chrono::Utc::now();
                        let remaining_secs = (expiration - now).num_seconds();

                        if remaining_secs > 0 {
                            let hours = remaining_secs / 3600;
                            let mins = (remaining_secs % 3600) / 60;

                            if hours > 0 {
                                format!("{}h {}m", hours, mins)
                            } else {
                                format!("{}m", mins)
                            }
                        } else {
                            "EXPIRED".to_string()
                        }
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                };

                Row::new(vec![
                    Cell::new(Text::from(status).alignment(Alignment::Center)),
                    Cell::new(Text::from(default_mark).alignment(Alignment::Center)),
                    Cell::new(
                        Text::from(account.account_name.clone()).alignment(Alignment::Center),
                    ),
                    Cell::new(Text::from(account.account_id.clone()).alignment(Alignment::Center)),
                    Cell::new(Text::from(account.role_name.clone()).alignment(Alignment::Center)),
                    Cell::new(Text::from(expiration_status).alignment(Alignment::Center)),
                ])
            })
            .collect();

        let header = Row::new(vec![
            Cell::new(Text::from("Status").alignment(Alignment::Center)),
            Cell::new(Text::from("Def").alignment(Alignment::Center)),
            Cell::new(Text::from("Account").alignment(Alignment::Center)),
            Cell::new(Text::from("Account ID").alignment(Alignment::Center)),
            Cell::new(Text::from("Role").alignment(Alignment::Center)),
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

        let table = Table::new(
            rows,
            [
                Constraint::Length(6),  // Status
                Constraint::Length(3),  // Default
                Constraint::Min(15),    // Account Name
                Constraint::Length(12), // Account ID
                Constraint::Min(15),    // Role Name
                Constraint::Length(10), // Expiration
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Accounts & Roles")
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

        // Status bar
        let selected_session_name = self
            .sessions_list_state
            .selected()
            .and_then(|idx| self.sso_sessions.get(idx))
            .map(|s| s.session_name.as_str())
            .unwrap_or("None");

        let status_text = if let Some(ref token) = self.sso_token {
            if token.is_expired() {
                format!(
                    "Session: {} | SSO Token: EXPIRED (Tab to Sessions pane, Enter to re-login)",
                    selected_session_name
                )
            } else {
                format!(
                    "Session: {} | SSO Token: Valid (expires in {})",
                    selected_session_name,
                    token.expiration_display()
                )
            }
        } else {
            format!(
                "Session: {} | Not logged in (Tab to Sessions pane, Enter to login)",
                selected_session_name
            )
        };

        let mut status_lines = vec![Line::from(status_text)];
        if let Some(ref msg) = self.status_message {
            status_lines.push(Line::from(msg.clone()));
        }

        let status = Paragraph::new(status_lines)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status, chunks[3]);

        // Help bar
        let help_bar = Paragraph::new(
            "q:quit | ?:help | Tab:switch pane | ↑↓/jk:navigate | Enter:toggle | Sessions: a:add e:edit d:delete | Accounts: p:profile d:default c:console r:refresh"
        )
            .style(Style::default().fg(catppuccin_color(self.theme.colors.subtext0)));
        f.render_widget(help_bar, chunks[4]);
    }

    fn draw_sessions_pane(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        let rows: Vec<Row> = self
            .sso_sessions
            .iter()
            .map(|session| {
                // Status indicator
                let status = if session.is_active { "🟢" } else { "🔴" };

                // Expiration status
                let expiration_status = if session.is_active {
                    if let Some(expiration) = session.token_expiration {
                        let now = chrono::Utc::now();
                        let remaining_secs = (expiration - now).num_seconds();

                        if remaining_secs > 0 {
                            let hours = remaining_secs / 3600;
                            let mins = (remaining_secs % 3600) / 60;

                            if hours > 0 {
                                format!("{}h {}m", hours, mins)
                            } else {
                                format!("{}m", mins)
                            }
                        } else {
                            "EXPIRED".to_string()
                        }
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                };

                Row::new(vec![
                    Cell::new(Text::from(status).alignment(Alignment::Center)),
                    Cell::new(
                        Text::from(session.session_name.clone()).alignment(Alignment::Center),
                    ),
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
                .title("SSO Sessions")
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
            Line::from("  Tab         - Switch between Sessions and Accounts panes"),
            Line::from("  ↑, k        - Move selection up"),
            Line::from("  ↓, j        - Move selection down"),
            Line::from(""),
            Line::from("Sessions Pane:"),
            Line::from("  Enter       - Login/Logout selected SSO session"),
            Line::from("  a           - Add new SSO session"),
            Line::from("  e           - Edit selected SSO session"),
            Line::from("  d           - Delete selected SSO session"),
            Line::from(""),
            Line::from("Accounts Pane:"),
            Line::from("  Enter       - Start/stop session (activate/invalidate credentials)"),
            Line::from("  p           - Edit profile name for selected role"),
            Line::from("  d           - Set selected role's profile as default"),
            Line::from("  c           - Open AWS Console in browser for selected role"),
            Line::from("  r           - Refresh account/role list"),
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

    fn draw_loading_screen(&self, f: &mut Frame) {
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
            loading_text.push(Line::from(Span::styled(
                "Browser opened automatically. If not, visit:",
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
            loading_text.push(Line::from(""));
            loading_text.push(Line::from(Span::styled(
                "Waiting for authorization...",
                Style::default().fg(Color::Gray),
            )));
        } else {
            // Generic loading message
            loading_text.push(Line::from(""));
            loading_text.push(Line::from(Span::styled(
                "Loading...",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
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
            Line::from("[default]"),
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
}
