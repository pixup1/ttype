use crate::term_colors::TERM_COLORS;
use std::fmt;

pub enum TermColorSupport {
	TrueColor,
	Ansi256,
	Ansi16,
	None,
}

// Returns the color support of the terminal
pub fn get_term_color_support() -> TermColorSupport {
	match std::env::var("COLORTERM") {
		Ok(val) => match val.as_str() {
			"truecolor" | "24bit" => TermColorSupport::TrueColor,
			"256color" | "8bit" => TermColorSupport::Ansi256,
			"ansi" | "standard" => TermColorSupport::Ansi16,
			_ => TermColorSupport::Ansi16 // I wasn't able to find a list of all possible values for $COLORTERM
		},
		Err(_) => match std::env::var("TERM") {
			Ok(val) => match val.as_str() {
				"xterm-256color" | "screen-256color" | "tmux-256color" | "rxvt-unicode-256color" | "linux" => TermColorSupport::Ansi256,
				_ => TermColorSupport::Ansi16
			},
			Err(_) => TermColorSupport::None
		}
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Color {
	r: u8,
	g: u8,
	b: u8,
}

impl Color {
	// New color from RGB
	pub const fn new_rgb(red: u8, green: u8, blue: u8) -> Color {
		Color {
			r: red,
			g: green,
			b: blue
		}
	}
	
	// New color from HSV (Hue is in degrees, saturation and value range from 0.0 to 1.0) (https://en.wikipedia.org/wiki/HSL_and_HSV#Color_conversion_formulae)
	pub fn new_hsv(hue: f32, saturation: f32, value: f32) -> Color {
		let c = value * saturation;
		let x = c * (1.0 - (((hue / 60.0) % 2.0) - 1.0).abs());
		let m = value - c;
		
		let nc = ((c + m) * 255.0) as u8;
		let nx = ((x + m) * 255.0) as u8;
		let no = (m * 255.0) as u8;
		
		match hue {
			h if (0.0..60.0).contains(&h) => Color{r: nc, g: nx, b: no},
			h if (60.0..120.0).contains(&h) => Color{r: nx, g: nc, b: no},
			h if (120.0..180.0).contains(&h) => Color{r: no, g: nc, b: nx},
			h if (180.0..240.0).contains(&h) => Color{r: no, g: nx, b: nc},
			h if (240.0..300.0).contains(&h) => Color{r: nx, g: no, b: nc},
			h if (300.0..360.0).contains(&h) => Color{r: nc, g: no, b: nx},
			_ => panic!("Hue must be between 0 and 360"),
		}
	}
	
	// New color from hex string
	pub fn new_hex(hex: &str) -> Color {
		let hex = hex.trim_start_matches("#");
		Color {
			r: u8::from_str_radix(&hex[0..2], 16).unwrap(),
			g: u8::from_str_radix(&hex[2..4], 16).unwrap(),
			b: u8::from_str_radix(&hex[4..6], 16).unwrap()
		}
	}
	
	// Convert color to hex
	pub fn to_hex(&self) -> String {
		format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
	}
	
	// Find the closest color from a list of colors
	fn closest_color(&self, colors: &[Color]) -> (Color, usize) {
		let mut best = 0;
		let mut best_score = std::f32::MAX;
		for i in 1..colors.len() {
			// Distance in RGB space (lower = better match) // TODO: Fix this
			let score = ((colors[i].r as f32 - self.r as f32).powf(2.0) + (colors[i].g as f32 - self.g as f32).powf(2.0) + (colors[i].b as f32 - self.b as f32).powf(2.0)).sqrt();
			if score < best_score {
				best = i;
				best_score = score;
			}
		}
		(colors[best].clone(), best)
	}
	
	// Convert color to escape sequence
	pub fn to_escape(&self, term_color_support: &TermColorSupport) -> Option<String> {
		match term_color_support {
			TermColorSupport::TrueColor => Some(format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)),
			TermColorSupport::Ansi256 => {
				let c_index = self.closest_color(&TERM_COLORS).1;
				Some(format!("\x1b[38;5;{}m", c_index))
			},
			TermColorSupport::Ansi16 => {
				let c_index = self.closest_color(&TERM_COLORS[0..16]).1;
				let prefix = if c_index > 7 {3} else {9};
				Some(format!("\x1b[{}{}m", prefix, c_index % 8))
			},
			TermColorSupport::None => None
		}
	}
}

pub struct ColoredText {
	chars: Vec<char>,
	colors: Vec<Color>,
	underline: Vec<bool>,
	bold: Vec<bool>,
}

impl ColoredText {
	// Create a new empty ColoredText
	pub fn new() -> ColoredText {
		ColoredText {
			chars: Vec::new(),
			colors: Vec::new(),
			underline: Vec::new(),
			bold: Vec::new(),
		}
	}
	
	// Create a new ColoredText from a string, a single color, and booleans for statuses
	pub fn new_text(text: &str, color: Color, underline: bool, bold: bool) -> ColoredText {
		let mut colored_text = ColoredText::new();
		
		colored_text.push_str(text, color, underline, bold);
		
		colored_text
	}
	
	// Get characters
	pub fn chars(&self) -> Vec<char> {
		self.chars.clone()
	}
	
	// Get colors
	pub fn colors(&self) -> Vec<Color> {
		self.colors.clone()
	}
	
	// Get underline status
	pub fn underline(&self) -> Vec<bool> {
		self.underline.clone()
	}
	
	// Set underline status
	pub fn set_underline(&mut self, idx: usize) {
		if idx < self.underline.len() {
			self.underline[idx] = true;
		}
	}
	
	// Get bold status
	pub fn bold(&self) -> Vec<bool> {
		self.bold.clone()
	}
	
	// Get text length
	pub fn len(&self) -> usize {
		self.chars.len()
	}
	
	// Get characters as a String
	pub fn text(&self) -> String {
		let mut s = String::new();
		
		for c in self.chars.clone() {
			s.push(c);
		}
		
		s
	}
	
	// Split text into a vector of Strings by a character
	pub fn split(&self, pattern_char: char) -> Vec<String> {
		let mut res: Vec<String> = Vec::new();
		let mut current = String::new();
		
		for c in self.chars.clone() {
			if c == pattern_char {
				res.push(current);
				current = String::new();
			} else {
				current.push(c);
			}
		}
		
		res
	}
	
	// Add single character to the end
	pub fn push(&mut self, ch: char, color: Color, underline: bool, bold: bool) {
		self.chars.push(ch);
		self.colors.push(color);
		self.underline.push(underline);
		self.bold.push(bold);
	}
	
	// Insert character at index
	pub fn insert(&mut self, idx: usize, ch: char, color: Color, underline: bool, bold: bool) {
		self.chars.insert(idx, ch);
		self.colors.insert(idx, color);
		self.underline.insert(idx, underline);
		self.bold.insert(idx, bold);
	}
	
	// Remove last character
	pub fn pop(&mut self) -> Option<(char, Color, bool, bool)> {
		if self.len() > 0 {
			Some((self.chars.pop().unwrap(), self.colors.pop().unwrap(), self.underline.pop().unwrap(), self.bold.pop().unwrap()))
		} else {
			None
		}
	}
	
	// Add single-colored string to the end of the ColoredText
	pub fn push_str(&mut self, s: &str, color: Color, underline: bool, bold: bool) {
		for c in s.chars() {
			self.chars.push(c);
			self.colors.push(color);
			self.underline.push(underline);
			self.bold.push(bold);
		}
	}
	
	// Add line returns to wrap text to a given max width
	pub fn word_wrap(&mut self, width: usize) {
		let mut line_length = 0;
		let mut last_word_start = 0;
		
		let chars: Vec<char> = self.chars().clone();
		let colors: Vec<Color> = self.colors().clone();
		let underline: Vec<bool> = self.underline.clone();
		let bold: Vec<bool> = self.bold.clone();
		
		let mut new_chars = Vec::new();
		let mut new_colors = Vec::new();
		let mut new_underline = Vec::new();
		let mut new_bold = Vec::new();
		
		//TODO: Make this less gross
		for i in 0..chars.len() {
			match chars[i] {
				'\n' => {
					new_chars.push('\n');
					new_colors.push(colors[i]);
					new_underline.push(underline[i]);
					new_bold.push(bold[i]);
					line_length = 0;
				},
				' ' => {
					new_chars.push(' ');
					new_colors.push(colors[i]);
					new_underline.push(underline[i]);
					new_bold.push(bold[i]);
					last_word_start = new_chars.len();
					line_length += 1;
				},
				_ => {
					if line_length + 1 > width  {
						if new_chars.len() - last_word_start >= width {
							new_chars.push('\n');
							new_colors.push(Color::new_rgb(255, 255, 255));
							new_underline.push(false);
							new_bold.push(false);
							last_word_start = new_chars.len();
							new_chars.push(chars[i]);
							new_colors.push(colors[i]);
							new_underline.push(underline[i]);
							new_bold.push(bold[i]);
							line_length = 1;
						} else {
							while new_chars[last_word_start - 1].is_whitespace() { // No whitespace on newlines
								new_chars.remove(last_word_start - 1);
								new_colors.remove(last_word_start - 1);
								new_underline.remove(last_word_start - 1);
								new_bold.remove(last_word_start - 1);
								last_word_start -= 1;
							}
							new_chars.insert(last_word_start, '\n');
							new_colors.insert(last_word_start, Color::new_rgb(255, 255, 255));
							new_underline.insert(last_word_start, false);
							new_bold.insert(last_word_start, false);
							last_word_start += 1;
							new_chars.push(chars[i]);
							new_colors.push(colors[i]);
							new_underline.push(underline[i]);
							new_bold.push(bold[i]);
							line_length = new_chars.len() - last_word_start;
						}
					} else {
						new_chars.push(chars[i]);
						new_colors.push(colors[i]);
						new_underline.push(underline[i]);
						new_bold.push(bold[i]);
						line_length += 1;
					}
				}
			}
		}
		
		self.chars = new_chars;
		self.colors = new_colors;
		self.underline = new_underline;
		self.bold = new_bold;
	}
}

impl fmt::Display for ColoredText {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", &self.text())
	}
}