use std::{env, fs::File, io::{stdout, Read}, panic, str::FromStr, time::Duration, cmp::max};

use getopts::Options;
use crossterm::{cursor, event::{self, KeyCode}, execute, terminal};
use crossterm::event::KeyCode::Char as CharCode;
use rand::Rng;

mod color;
mod pixels;
mod term_colors;
mod utils;
mod game;

use color::Color;
use pixels::*;
use color::*;
use utils::nth_char_idx;

const LANGUAGES_PATH: &str = "static/languages";
const QUOTES_PATH: &str = "static/quotes";
const UNTYPED_COLOR: Color = Color::new_rgb(80, 80, 80);
const TYPED_COLOR: Color = Color::new_rgb(255, 255, 255);
const WRONG_COLOR: Color = Color::new_rgb(255, 0, 0);

#[derive(Debug)]
enum DictEntry {
	Word(String),
	Quote{text: String, source: String},
}

#[derive(Debug, PartialEq)]
enum GameMode {
	CountedWords{number_of_words: u32},
	TimedWords{time: u32},
	Quote,
	Text{text: String}
}

fn used_text_width(twidth: usize) -> usize {
	let factor = match twidth {
		0..50 => 0.8,
		50..100 => 0.7,
		100..170 => 0.6,
		_ => 0.5
	};
	return (twidth as f32 * factor) as usize;
}

fn correct_combine(typed: &str, text: &str) -> ColoredText {
	let mut out = ColoredText::new();
	
	let typed_words: Vec<&str> = typed.split_whitespace().collect();
	let text_words: Vec<&str> = text.split_whitespace().collect();
	
	for i in 0..text_words.len() {
		if i < typed_words.len() {
			let text_chars: Vec<char> = text_words[i].chars().collect();
			let typed_chars: Vec<char> = typed_words[i].chars().collect();
			
			for j in 0..text_chars.len() {
				if typed_chars.get(j).unwrap_or(&' ').to_owned() == text_chars[j].to_owned() {
					out.push(typed_chars[j], TYPED_COLOR, false, false);
				} else if let Some(c) = typed_chars.get(j) {
					out.push(c.to_owned(), WRONG_COLOR, false, true);
				} else {
					out.push(text_chars[j], UNTYPED_COLOR, false, false);
				}
			}
			
			// This is for when the player types longer than the word
			for j in text_chars.len()..typed_chars.len() {
				out.push(typed_chars[j], WRONG_COLOR, false, true);
			}
		} else {
			out.push_str(text_words[i], UNTYPED_COLOR, false, false);
		}
		out.push(' ', UNTYPED_COLOR, false, false);
	}
	
	out
}

fn show_cursor(to_print: &mut ColoredText, typed: &str, text: &str, cursor_pos: usize) {
	let typed_words: Vec<&str> = typed.split_whitespace().collect();
	let text_words: Vec<&str> = text.split_whitespace().collect();
	
	let mut diff = 0;
	
	let space_terminated = typed.chars().last().unwrap_or('a').eq(&' ');
	if typed_words.len() > if space_terminated {0} else {1} {
		for i in 0..(typed_words.len() - if space_terminated {0} else {1}) {
			diff += max(text_words[i].len() as i32 - typed_words[i].len() as i32, 0);
		}
	}
	
	to_print.set_underline((cursor_pos as i32 + diff) as usize);
}

fn print_usage(program: &str, opts: Options) {
	let brief = format!("Usage: {} [options]", program);
	print!("{}", opts.usage(&brief));
}

fn main() {
	// getopts things
	let args: Vec<String> = env::args().collect();
	let program = args[0].clone();
	let mut opts = Options::new();
	
	opts.optopt("l", "lang", "which language to use", "LANGUAGE");
	opts.optopt("w", "words", "use the provided number of words", "INTEGER");
	opts.optopt("d", "duration", "play for the provided duration", "SECONDS");
	opts.optflag("q", "quotes", "use quotes");
	opts.optopt("t", "text", "use provided text", "TEXT");
	opts.optopt("f", "file", "use text from provided file", "PATH");
	opts.optflag("h", "help", "print this help menu");
	let matches = match opts.parse(&args[1..]) {
		Ok(m) => { m }
		Err(f) => { panic!("{}", f.to_string()) }
	};
	
	if matches.opt_present("h") {
		print_usage(&program, opts);
		return;
	}
	
	let lang = match matches.opt_str("l") {
		Some(l) => l,
		None => "english".to_string()
	};
	
	let game_mode = {
		let mut selected = false;
		let mut game_mode = GameMode::CountedWords{number_of_words: 30};
		if matches.opt_present("q") {
			game_mode = GameMode::Quote;
			selected = true;
		}
		if let Some(t) = matches.opt_str("t") {
			game_mode = GameMode::Text{text: t};
			if selected {
				panic!("Only one game mode can be selected at a time.");
			}
			selected = true;
		}
		if let Some(path) = matches.opt_str("f") {
			let mut file = File::open(&path).expect(&format!("Can't open {}. Does the file exist ?", path));
			let mut contents = String::new();
			file.read_to_string(&mut contents).unwrap();
			game_mode = GameMode::Text{text: contents};
			if selected {
				panic!("Only one game mode can be selected at a time.");
			}
			selected = true;
		}
		if let Some(d) = matches.opt_str("d") {
			game_mode = GameMode::TimedWords{time: d.parse().unwrap()};
			if selected {
				panic!("Only one game mode can be selected at a time.");
			}
			selected = true;
		}
		if let Some(w) = matches.opt_str("w") {
			game_mode = GameMode::CountedWords{number_of_words: w.parse().unwrap()};
			if selected {
				panic!("Only one game mode can be selected at a time.");
			}
		}
		game_mode
	};
	
	let dict_dir = if cfg!(debug_assertions) {
		env::var("CARGO_MANIFEST_DIR").unwrap()
	} else {
		//TODO: find where to put the dictionnaries for installs
		"".to_string()
	};
	
	let dict = {
		let path = format!("{}/{}/{}.json", dict_dir, if game_mode == GameMode::Quote { QUOTES_PATH } else { LANGUAGES_PATH }, lang);
		let mut file = File::open(&path).expect("That language doesn't exist.");
		let mut contents = String::new();
		file.read_to_string(&mut contents).unwrap();
		let parsed = jzon::parse(&mut contents).unwrap();
		let mut dict: Vec<DictEntry> = Vec::new();
		if game_mode == GameMode::Quote {
			for quote in parsed["quotes"].as_array().unwrap() {
				dict.push(DictEntry::Quote{
					text: quote["text"].as_str().unwrap().to_string(),
					source: quote["source"].as_str().unwrap().to_string()
				});
			}
		} else {
			for word in parsed["words"].as_array().unwrap() {
				dict.push(DictEntry::Word(word.as_str().unwrap().to_string()));
			}
		}
		dict
	};
	
	let term_color_support = get_term_color_support();
	let mut rng = rand::rng();
	let mut stdout = stdout();

	terminal::enable_raw_mode().unwrap();
	execute!(stdout, cursor::Hide).unwrap();
	execute!(stdout, terminal::DisableLineWrap).unwrap();
	execute!(stdout, terminal::EnterAlternateScreen).unwrap();
	
	// This will be called on a panic so the terminal doesn't stay all messed up
	panic::set_hook(Box::new(|info| {
		let mut stdout = std::io::stdout();
		
		terminal::disable_raw_mode().unwrap();
		execute!(stdout, terminal::EnableLineWrap).unwrap();
		execute!(stdout, terminal::LeaveAlternateScreen).unwrap();
		execute!(stdout, cursor::Show).unwrap();
		
		println!("{}", std::backtrace::Backtrace::force_capture());
		println!("{}", info);
	}));
	
	let mut cpt_it = 0;
	
	'main: loop {
		let source = String::new();
		
		let text = match game_mode {
			GameMode::CountedWords{number_of_words} => {
				let mut text = String::new();
				for _ in 0..number_of_words {
					match &dict[rng.random_range(0..dict.len())] {
						DictEntry::Word(w) => {
							text.push_str(w);
							text.push_str(" ");
						},
						_ => {}
					}
				}
				text
			},
			GameMode::TimedWords{time} => {
				let mut text = String::new();
				for _ in 0..100 {
					match &dict[rng.random_range(0..dict.len())] {
						DictEntry::Word(w) => {
							text.push_str(w);
							text.push_str(" ");
						},
						_ => {}
					}
				}
				text
			},
			GameMode::Quote => {
				let mut text = String::new();
				match &dict[rng.random_range(0..dict.len())] {
					DictEntry::Quote{text: t, source: s} => {
						text.push_str(t);
						let source = String::from_str(s).unwrap();
					},
					_ => {}
				}
				text
			},
			GameMode::Text{ref text} => {
				if cpt_it > 0 {
					break 'main;
				}
				text.clone()
			}
		};
		
		let text_words: Vec<&str> = text.split_whitespace().collect();
		let mut typed = String::new();
		let mut cursor_pos = 0;
		
		'game: loop {
			let typed_words: Vec<&str> = typed.split_whitespace().collect();
			
			if typed_words.len() > text_words.len()
			|| (typed_words.len() == text_words.len() && (typed.ends_with(' ')
			|| typed_words.last().unwrap_or(&"").eq(text_words.last().unwrap_or(&"")))) {
				break 'game;
			}
			
			let tsize = terminal::size().unwrap();
			let mut pixels = Pixels::new((tsize.0 as usize, tsize.1 as usize));
			
			let mut to_print = correct_combine(&typed, &text);
			show_cursor(&mut to_print, &typed, &text, cursor_pos);
			
			let text_width = used_text_width(tsize.0 as usize);
			
			//TODO: Handle newlines
			to_print.word_wrap(text_width);
			
			let start_pos = {
				if to_print.text().matches('\n').count() > 0 {
					(tsize.0 as usize - text_width) / 2
				} else {	
					(tsize.0 as usize - to_print.text().len()) / 2
				}
			};
			
			pixels.print_color(
				&to_print,
				(start_pos, tsize.1 as usize / 2),
				HCentering::Left, // We don't center on the middle as that could cause some jitters
				VCentering::Middle
			);
			
			pixels.render(&term_color_support);
			
			// Get events
			let mut key_events:Vec<event::KeyEvent> = Vec::new();
			if event::poll(Duration::from_secs(0)).unwrap() { // Event is available
				while event::poll(Duration::from_secs(0)).unwrap() {
					if let event::Event::Key(key_event) = event::read().unwrap() {
						key_events.push(key_event);
					}
				}
			} else { // No event available; wait for one
				if let event::Event::Key(key_event) = event::read().unwrap() {
					key_events.push(key_event);
				}
			}
			
			// Process events
			for e in key_events {
				if e.code == CharCode('r') && e.modifiers == event::KeyModifiers::CONTROL{
					break 'game;
				} else if e.code == CharCode('c') && e.modifiers == event::KeyModifiers::CONTROL{
					break 'main;
				} else if e.code == KeyCode::Backspace {
					if cursor_pos > 0 {
						typed.remove(
							nth_char_idx(&typed, cursor_pos - 1)
						);
						cursor_pos -= 1;
					}
				} else if e.code == KeyCode::Delete {
					if cursor_pos < typed.chars().count() {
						typed.remove(
							nth_char_idx(&typed, cursor_pos)
						);
					}
				} else if e.code == KeyCode::Left {
					if cursor_pos > 0 {
						cursor_pos -= 1;
					}
				} else if e.code == KeyCode::Right {
					if cursor_pos < typed.chars().count() {
						cursor_pos += 1;
					}
				} else if let CharCode(c) = e.code {
					let chars: Vec<char> = typed.chars().collect();
					
					if e.code != CharCode(' ')
						|| (cursor_pos >= chars.len() && chars.last().unwrap_or(&' ').to_owned() != ' ')
						|| (cursor_pos < chars.len() && chars[cursor_pos - 1] != ' ' && chars[cursor_pos] != ' ') {
						
						if cursor_pos >= chars.len() {
							typed.push(c);
						} else {
							typed.insert(
								nth_char_idx(&typed, cursor_pos),
								c,
							);
						}
						cursor_pos += 1;
					}
				}
			}
			
			cpt_it += 1;
		}
	}
	
	terminal::disable_raw_mode().unwrap();
	execute!(stdout, terminal::EnableLineWrap).unwrap();
	execute!(stdout, terminal::LeaveAlternateScreen).unwrap();
	execute!(stdout, cursor::Show).unwrap();
}