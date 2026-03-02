use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
// 本地开发使用 nesium_core，发布时使用 nesium
use nesium_core::{
    controller::Button,
    ppu::buffer::ColorFormat,
    Nes, NesBuilder,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{
    io::{self, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

const SCREEN_WIDTH: usize = 256;
const SCREEN_HEIGHT: usize = 240;
const MIN_TERMINAL_WIDTH: u16 = 100;
const MIN_TERMINAL_HEIGHT: u16 = 35;

#[derive(Parser, Debug)]
#[command(name = "nesium-cli")]
#[command(about = "NES emulator for terminal with iTerm2 image support")]
struct Args {
    #[arg(help = "Path to the NES ROM file")]
    rom: PathBuf,
}

struct EmulatorApp {
    nes: Nes,
    fps: f32,
    frame_count: u32,
    fps_update_time: Instant,
    frame_buffer: Vec<u8>,
    buttons: [bool; 8],
    rgba_buffer: Vec<u8>,
    png_buffer: Vec<u8>,
    temp_buffer: Vec<u8>,
    pressed_keys: std::collections::HashMap<char, Instant>,
}

impl EmulatorApp {
    fn new(rom_path: PathBuf) -> Result<Self> {
        let mut nes = NesBuilder::new()
            .format(ColorFormat::Rgb555)
            .build();

        nes.load_cartridge_from_file(&rom_path)?;

        Ok(Self {
            nes,
            fps: 0.0,
            frame_count: 0,
            fps_update_time: Instant::now(),
            frame_buffer: vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 2],
            buttons: [false; 8],
            rgba_buffer: vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
            png_buffer: Vec::with_capacity(256 * 1024),
            temp_buffer: Vec::with_capacity(SCREEN_WIDTH * SCREEN_HEIGHT * 4 + SCREEN_HEIGHT),
            pressed_keys: std::collections::HashMap::new(),
        })
    }

    fn set_button(&mut self, btn: Button, pressed: bool) {
        let idx = btn as usize;
        if idx < 8 {
            self.buttons[idx] = pressed;
        }
    }

    fn update(&mut self) {
        self.nes.set_button(0, Button::A, self.buttons[0]);
        self.nes.set_button(0, Button::B, self.buttons[1]);
        self.nes.set_button(0, Button::Select, self.buttons[2]);
        self.nes.set_button(0, Button::Start, self.buttons[3]);
        self.nes.set_button(0, Button::Up, self.buttons[4]);
        self.nes.set_button(0, Button::Down, self.buttons[5]);
        self.nes.set_button(0, Button::Left, self.buttons[6]);
        self.nes.set_button(0, Button::Right, self.buttons[7]);

        let _audio = self.nes.run_frame(false);

        let now = Instant::now();
        self.frame_count += 1;
        let fps_update_duration = now.duration_since(self.fps_update_time);
        if fps_update_duration >= Duration::from_secs(1) {
            self.fps = self.frame_count as f32 / fps_update_duration.as_secs_f32();
            self.frame_count = 0;
            self.fps_update_time = now;
        }
    }

    fn render_rgba_to_buffer(&mut self) {
        let frame = self.nes.try_render_buffer();
        if let Some(buffer) = frame {
            self.frame_buffer.copy_from_slice(buffer);
        }
        
        let mut idx = 0;
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let src_idx = (y * SCREEN_WIDTH + x) * 2;
                
                if src_idx + 1 < self.frame_buffer.len() {
                    let pixel = (self.frame_buffer[src_idx] as u16) | ((self.frame_buffer[src_idx + 1] as u16) << 8);
                    let r = ((pixel >> 10) & 0x1F) as u8;
                    let g = ((pixel >> 5) & 0x1F) as u8;
                    let b = (pixel & 0x1F) as u8;
                    
                    self.rgba_buffer[idx] = (r << 3) | (r >> 2);
                    self.rgba_buffer[idx + 1] = (g << 3) | (g >> 2);
                    self.rgba_buffer[idx + 2] = (b << 3) | (b >> 2);
                    self.rgba_buffer[idx + 3] = 255;
                } else {
                    self.rgba_buffer[idx] = 0;
                    self.rgba_buffer[idx + 1] = 0;
                    self.rgba_buffer[idx + 2] = 0;
                    self.rgba_buffer[idx + 3] = 255;
                }
                idx += 4;
            }
        }
    }

    fn generate_iterm2_image(&mut self, width: u32, height: u32) -> String {
        self.render_rgba_to_buffer();

        self.png_buffer.clear();
        encode_png_to_buffer(&self.rgba_buffer, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, &mut self.png_buffer, &mut self.temp_buffer);

        let base64_data = base64::encode(&self.png_buffer);

        format!(
            "\x1b]1337;File=inline=1;size={};width={}px;height={}px;preserveAspectRatio=1:{}\x07",
            base64_data.len(),
            width,
            height,
            base64_data
        )
    }
}

fn encode_png_to_buffer(rgba_data: &[u8], width: u32, height: u32, output: &mut Vec<u8>, temp_buffer: &mut Vec<u8>) {
    output.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    
    create_ihdr_chunk_to_buffer(width, height, output);
    create_idat_chunk_to_buffer(rgba_data, width, height, output, temp_buffer);
    create_iend_chunk_to_buffer(output);
}

fn create_ihdr_chunk_to_buffer(width: u32, height: u32, output: &mut Vec<u8>) {
    let len = 13u32;
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(b"IHDR");
    
    let mut data = [0u8; 13];
    data[0..4].copy_from_slice(&width.to_be_bytes());
    data[4..8].copy_from_slice(&height.to_be_bytes());
    data[8] = 8;
    data[9] = 6;
    data[10] = 0;
    data[11] = 0;
    data[12] = 0;
    output.extend_from_slice(&data);
    
    let crc = crc32(&output[output.len() - 17..output.len()]);
    output.extend_from_slice(&crc.to_be_bytes());
}

fn create_idat_chunk_to_buffer(rgba_data: &[u8], width: u32, height: u32, output: &mut Vec<u8>, temp_buffer: &mut Vec<u8>) {
    let w = width as usize;
    let h = height as usize;
    let row_size = w * 4 + 1;
    
    temp_buffer.clear();
    temp_buffer.resize(h * row_size, 0);
    
    for y in 0..h {
        let row_start = y * row_size;
        temp_buffer[row_start] = 0;
        
        for x in 0..w {
            let src_idx = (y * w + x) * 4;
            let dst_idx = row_start + 1 + x * 4;
            temp_buffer[dst_idx] = rgba_data[src_idx];
            temp_buffer[dst_idx + 1] = rgba_data[src_idx + 1];
            temp_buffer[dst_idx + 2] = rgba_data[src_idx + 2];
            temp_buffer[dst_idx + 3] = rgba_data[src_idx + 3];
        }
    }
    
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(temp_buffer, 1);
    
    let len = compressed.len() as u32;
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(b"IDAT");
    output.extend_from_slice(&compressed);
    
    let crc_start = output.len() - compressed.len() - 4;
    let crc = crc32(&output[crc_start..output.len()]);
    output.extend_from_slice(&crc.to_be_bytes());
}

fn create_iend_chunk_to_buffer(output: &mut Vec<u8>) {
    output.extend_from_slice(&[0, 0, 0, 0]);
    output.extend_from_slice(b"IEND");
    let crc = crc32(b"IEND");
    output.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    const CRC_TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut n = 0;
        while n < 256 {
            let mut c = n as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 {
                    0xEDB88320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            table[n] = c;
            n += 1;
        }
        table
    };
    
    let mut c = !0u32;
    for &byte in data {
        c = CRC_TABLE[((c ^ byte as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

fn run_app(rom_path: PathBuf) -> Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = EmulatorApp::new(rom_path)?;
    let mut screen_rect: Option<Rect> = None;

    loop {
        terminal.draw(|f| {
            let area = f.area();
            if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
                let paragraph = Paragraph::new(vec![
                    Line::from("Terminal too small."),
                    Line::from(format!("Required: {}x{}", MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT)),
                    Line::from(format!("Current: {}x{}", area.width, area.height)),
                    Line::from("Please resize your terminal."),
                ])
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Red));
                f.render_widget(paragraph, area);
                return;
            }

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([
                    Constraint::Percentage(70),
                    Constraint::Percentage(30),
                ])
                .split(area);

            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(30),
                ])
                .split(chunks[0]);

            let header = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("NESium CLI", Style::default().fg(Color::Cyan)),
                    Span::raw(" | "),
                    Span::styled(format!("FPS: {:.1}", app.fps), Style::default().fg(Color::Green)),
                ]),
            ])
            .block(Block::default().borders(Borders::ALL).title("Status"));
            f.render_widget(header, left_chunks[0]);

            let screen_block = Block::default().borders(Borders::ALL).title("Screen (iTerm2)");
            let inner = screen_block.inner(left_chunks[1]);
            screen_rect = Some(inner);
            f.render_widget(screen_block, left_chunks[1]);

            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(12),
                    Constraint::Min(10),
                ])
                .split(chunks[1]);

            let controls = Paragraph::new(vec![
                Line::from("Controls:"),
                Line::from(""),
                Line::from("  WASD: Direction"),
                Line::from("  J: A, K: B"),
                Line::from("  L: Select, ;: Start"),
                Line::from("  Q: Quit"),
            ])
            .block(Block::default().borders(Borders::ALL).title("Help"));
            f.render_widget(controls, right_chunks[0]);

            let info = Paragraph::new(vec![
                Line::from("iTerm2 Image Mode"),
                Line::from(""),
                Line::from("Requires iTerm2 terminal"),
                Line::from("with image support."),
            ])
            .block(Block::default().borders(Borders::ALL).title("Info"));
            f.render_widget(info, right_chunks[1]);
        })?;

        // 处理输入事件，更新持久化的按键状态
        while event::poll(Duration::from_millis(0))? {
            if let Ok(Event::Key(key)) = event::read() {
                if let KeyCode::Char(c) = key.code {
                    if c == 'q' {
                        disable_raw_mode()?;
                        io::stdout().execute(LeaveAlternateScreen)?;
                        return Ok(());
                    }

                    match key.kind {
                        KeyEventKind::Press | KeyEventKind::Repeat => {
                            // 更新按键的最后活动时间
                            app.pressed_keys.insert(c, Instant::now());
                        }
                        KeyEventKind::Release => {
                            app.pressed_keys.remove(&c);
                        }
                        _ => {}
                    }
                }
            }
        }

        // 清除超过 200ms 没有收到 Repeat 事件的按键
        // 按住按键时应该持续收到 Repeat 事件
        let now = Instant::now();
        app.pressed_keys.retain(|_, last_time| now.duration_since(*last_time) < Duration::from_millis(200));
        
        // 根据持久化的按键状态更新按钮
        let up = app.pressed_keys.contains_key(&'w');
        let down = app.pressed_keys.contains_key(&'s');
        let left = app.pressed_keys.contains_key(&'a');
        let right = app.pressed_keys.contains_key(&'d');

        // 防止同时按下相反方向键（NES 游戏通常不支持）
        let (up, down) = if up && down {
            (true, false) // 优先上
        } else {
            (up, down)
        };
        let (left, right) = if left && right {
            (true, false) // 优先左
        } else {
            (left, right)
        };

        app.set_button(Button::Up, up);
        app.set_button(Button::Down, down);
        app.set_button(Button::Left, left);
        app.set_button(Button::Right, right);
        app.set_button(Button::A, app.pressed_keys.contains_key(&'j'));
        app.set_button(Button::B, app.pressed_keys.contains_key(&'k'));
        app.set_button(Button::Select, app.pressed_keys.contains_key(&'l'));
        app.set_button(Button::Start, app.pressed_keys.contains_key(&';'));

        // 更新 NES 模拟器（在渲染前）
        let frame_start = Instant::now();
        app.update();

        // 渲染图片
        if let Some(rect) = screen_rect {
            let char_width = 8;
            let char_height = 16;
            let img_width = (rect.width as u32).saturating_sub(2) * char_width;
            let img_height = (rect.height as u32).saturating_sub(2) * char_height;

            if img_width > 0 && img_height > 0 {
                let image_sequence = app.generate_iterm2_image(img_width, img_height);

                crossterm::queue!(
                    io::stdout(),
                    crossterm::cursor::MoveTo(rect.x + 1, rect.y + 1)
                )?;
                print!("{}", image_sequence);
                io::stdout().flush()?;
            }
        }
        
        // 限制帧率为 60 FPS
        let frame_duration = frame_start.elapsed();
        let target_duration = Duration::from_nanos(16_666_667); // ~60 FPS
        if frame_duration < target_duration {
            std::thread::sleep(target_duration - frame_duration);
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.rom.exists() {
        eprintln!("Error: ROM file not found: {}", args.rom.display());
        std::process::exit(1);
    }

    run_app(args.rom)
}
