use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use myclaw_common::{ClientMessage, ServerMessage};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use tokio::sync::mpsc;

pub enum ChatEntry {
    User(String),
    Bot(String),
    System(String),
}

struct App {
    input: String,
    messages: Vec<ChatEntry>,
    scroll: u16,
    gateway_connected: bool,
    outbound_tx: mpsc::Sender<ClientMessage>,
}

impl App {
    fn new(tx: mpsc::Sender<ClientMessage>) -> Self {
        Self {
            input: String::new(),
            messages: vec![ChatEntry::System("Welcome to MyClaw!".into())],
            scroll: 0,
            gateway_connected: false,
            outbound_tx: tx,
        }
    }
}

pub async fn run(
    mut inbound_rx: mpsc::Receiver<ServerMessage>,
    outbound_tx: mpsc::Sender<ClientMessage>,
) -> anyhow::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(outbound_tx);
    let tick = std::time::Duration::from_millis(50);

    loop {
        terminal.draw(|f| draw(f, &app))?;
        while let Ok(msg) = inbound_rx.try_recv() {
            handle_server_msg(&mut app, msg);
        }
        if !event::poll(tick)? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                (KeyCode::Enter, _) => send_input(&mut app).await,
                (KeyCode::Char(c), _) => app.input.push(c),
                (KeyCode::Backspace, _) => { app.input.pop(); }
                (KeyCode::Up, _) => app.scroll = app.scroll.saturating_add(1),
                (KeyCode::Down, _) => app.scroll = app.scroll.saturating_sub(1),
                _ => {}
            }
        }
    }

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    Ok(())
}

async fn send_input(app: &mut App) {
    let text = app.input.trim().to_string();
    if text.is_empty() {
        return;
    }
    app.messages.push(ChatEntry::User(text.clone()));
    app.input.clear();
    let msg = ClientMessage::new_chat(&text);
    let _ = app.outbound_tx.send(msg).await;
}

fn handle_server_msg(app: &mut App, msg: ServerMessage) {
    match msg {
        ServerMessage::ChatReply { content, done, .. } => {
            if done {
                app.messages.push(ChatEntry::Bot(content));
            } else {
                // Streaming: append to last bot message or create new
                if let Some(ChatEntry::Bot(ref mut s)) = app.messages.last_mut() {
                    s.push_str(&content);
                } else {
                    app.messages.push(ChatEntry::Bot(content));
                }
            }
        }
        ServerMessage::Error { message } => {
            app.messages.push(ChatEntry::System(format!("Error: {message}")));
        }
        ServerMessage::Status { gateway_connected } => {
            app.gateway_connected = gateway_connected;
            let status = if gateway_connected { "Gateway connected" } else { "Gateway disconnected" };
            app.messages.push(ChatEntry::System(status.into()));
        }
        ServerMessage::Pong => {}
    }
}

fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Status bar
    let status = if app.gateway_connected { "CONNECTED" } else { "DISCONNECTED" };
    let status_line = Paragraph::new(format!(" MyClaw | Gateway: {status} | Ctrl+C to quit"))
        .style(Style::default().bg(Color::Blue).fg(Color::White));
    f.render_widget(status_line, chunks[0]);

    // Chat messages
    let items: Vec<ListItem> = app.messages.iter().map(|entry| {
        match entry {
            ChatEntry::User(s) => ListItem::new(format!("> {s}"))
                .style(Style::default().fg(Color::Cyan)),
            ChatEntry::Bot(s) => ListItem::new(format!("  {s}"))
                .style(Style::default().fg(Color::Green)),
            ChatEntry::System(s) => ListItem::new(format!("* {s}"))
                .style(Style::default().fg(Color::Yellow)),
        }
    }).collect();
    let chat = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Chat"));
    f.render_widget(chat, chunks[1]);

    // Input box
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .wrap(Wrap { trim: false });
    f.render_widget(input, chunks[2]);
}
