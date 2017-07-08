#![cfg_attr(feature = "nightly", feature(io))]

extern crate libgo;
extern crate liner;
extern crate termion;

mod menu;

use std::{cmp, fmt, io, thread, time};
use std::io::Write;

use libgo::game::board::Board;
use libgo::game::{Game, Handicap};
use libgo::game::player::Player as LibPlayer;
use libgo::gtp::command::Command;
use libgo::gtp::engine::Engine;
use liner::Context;
use termion::clear;
use termion::color::{self, AnsiValue};
use termion::cursor::Goto;
use termion::raw::{IntoRawMode, RawTerminal};

use menu::Menu;

fn main() {
    println!("Welcome to Redox Go\r\n");

    let settings = Settings::request_new();
    let mut game = GameHandle::new(settings);
    game.start();
}

struct Settings {
    black: Player,
    white: Player,
    board_size: usize,
    handicap: usize,
    is_gtp_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            black: Player::Human,
            white: Player::Human,
            board_size: 19,
            handicap: 0,
            is_gtp_mode: false,
        }
    }
}

impl Settings {
    fn request_new() -> Self {
        let is_gtp_mode = Menu {
            prompt: "play in Go Text Protocol mode:".to_string(),
            options: vec![false, true],
            default: 0,
        }.select_option();

        if is_gtp_mode {
            return Settings { is_gtp_mode, .. Default::default() };
        }

        let black = Menu {
            prompt: "black player:".to_string(),
            options: vec![Player::Human, Player::Computer],
            default: 0,
        }.select_option();

        let white = Menu {
            prompt: "white player:".to_string(),
            options: vec![Player::Human, Player::Computer],
            default: 0,
        }.select_option();

        let board_size =  Menu {
            prompt: "board size:".to_string(),
            options: vec![9, 13, 19],
            default: 2,
        }.select_option();

        let handicap =  Menu {
            prompt: "handicap:".to_string(),
            options: vec![0, 0, 2, 3, 4, 5, 6, 7, 8, 9],
            default: 0,
        }.select_option();

        Settings { is_gtp_mode, black, white, board_size, handicap }
    }
}

struct GameHandle {
    gtp: Engine,
    game: Game,
    prompt: Context,
    settings: Settings,
    info_box: String,
    should_quit: bool,
}

impl GameHandle {
    fn new(settings: Settings) -> Self {
        let mut gtp = Engine::new();
        gtp.register_all_commands();

        let mut game = Game::with_board_size(settings.board_size).expect("invalid board size");
        if settings.handicap > 0 {
            game.place_handicap(settings.handicap, Handicap::Fixed).expect("invalid handicap");
        }

        let prompt = Context::new();
        GameHandle { gtp, game, prompt, settings, info_box: String::new(), should_quit: false }
    }
}

impl GameHandle {
    pub fn start(&mut self) {
        let stdout = io::stdout();
        let mut stdout = stdout.lock().into_raw_mode().unwrap();

        self.start_interactive_mode(&mut stdout);

        reset_screen(&mut stdout);
    }

    fn start_interactive_mode(&mut self, stdout: &mut RawTerminal<io::StdoutLock>) {
        if self.settings.is_gtp_mode {
            self.info_box = "\r\n Enter 'list_commands' for a full list of options.".to_string();
        }

        loop {
            if self.should_quit { return; }

            reset_screen(stdout);
            draw_board(self.game.board());
            self.draw_info_box(stdout);

            if self.settings.is_gtp_mode {
                self.read_prompt_gtp();
            } else {
                self.read_prompt();
            }
        }
    }

    fn draw_info_box(&mut self, stdout: &mut RawTerminal<io::StdoutLock>) {
        let board_size = self.game.board().size();
        let below_the_board = board_size as u16 + 3;
        let column_offset = 2 * board_size as u16 + 8;
        let mut line_number = 0;

        for line in self.info_box.lines() {
            line_number += 1;
            write!(stdout, "{}{}", Goto(column_offset, line_number), line).expect("failed write");
        }

        let prompt_line = cmp::max(line_number, below_the_board);
        write!(stdout, "{}", Goto(1, prompt_line)).expect("goto failed");
    }

    fn read_prompt_gtp(&mut self) {
        let line = self.prompt.read_line("GTP> ", &mut |_event_handler| {})
                .expect("failed to read prompt");

        if let Some(command) = Command::from_line(&line) {
            self.prompt.history.push(line.into()).unwrap();

            self.info_box = self.gtp.exec(&mut self.game, &command).to_string();

            if command.name == "quit" {
                self.should_quit = true;
            }
        }
    }

    fn read_prompt(&mut self) {
        if self.game.is_over() {
            self.settings.is_gtp_mode = true;
            return;
        }
        let color = self.game.player_turn();
        match self.get_player_settings(color) {
            Player::Computer => {
                self.game.genmove_random(color);
                thread::sleep(time::Duration::from_millis(50));
            },
            Player::Human => {
                let prompt_text = format!(">play {} ", color);
                let line = self.prompt.read_line(prompt_text, &mut |_event_handler| {})
                        .expect("failed to read prompt");

                if line == "quit" {
                    self.should_quit = true;
                    return;
                }

                let prefix = format!("play {} ", color);
                if let Some(command) = Command::from_line(&(prefix + &line)) {
                    let response = self.gtp.exec(&mut self.game, &command);
                    self.info_box = response.to_string();
                }
            }
        }
    }

    fn get_player_settings(&self, player: LibPlayer) -> Player {
        match player {
            LibPlayer::Black => self.settings.black,
            LibPlayer::White => self.settings.white,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Player {
    Human,
    Computer,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let buf = match *self {
            Player::Human => "human",
            Player::Computer => "computer",
        };
        write!(f, "{}", buf)
    }
}

fn reset_screen(stdout: &mut RawTerminal<io::StdoutLock>) {
    write!(stdout, "{}{}", clear::All, Goto(1, 1)).expect("reset_screen: failed write");
    stdout.flush().expect("reset_screen: failed to flush stdout");
}

/// Writes a colored version of showboard to stdout using termion.
pub fn draw_board(board: &Board) {
    let stdout = io::stdout();
    let mut stdout = stdout.lock().into_raw_mode().unwrap();
    let mut board = board.to_ascii();
    board.push_str("\r\n");

    write!(stdout, "{}", color::Bg(AnsiValue::grayscale(11))).unwrap();
    for character in board.chars() {
        match character {
            'x' => {
                write!(stdout, "{}", color::Fg(AnsiValue::grayscale(0))).unwrap();
                stdout.write("●".as_bytes()).unwrap();
            },
            'o' => {
                write!(stdout, "{}", color::Fg(AnsiValue::grayscale(23))).unwrap();
                stdout.write("●".as_bytes()).unwrap();
            },
            '\n' => {
                write!(stdout, "{}", color::Bg(color::Reset)).unwrap();
                stdout.write(character.to_string().as_bytes()).unwrap();
                write!(stdout, "{}", color::Bg(AnsiValue::grayscale(11))).unwrap();
            },
            _ => {
                write!(stdout, "{}", color::Fg(AnsiValue::grayscale(23))).unwrap();
                stdout.write(character.to_string().as_bytes()).unwrap();
            }
        }
    }

    write!(stdout, "{}{}", color::Fg(color::Reset), color::Bg(color::Reset)).unwrap();
    stdout.flush().unwrap();
}
