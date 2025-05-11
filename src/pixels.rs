use crate::color::*;

pub enum HCentering {
	Left,
	Center,
	Right
}

pub enum VCentering {
	Top,
	Middle,
	Bottom
}

pub struct Pixels {
	pub size: (usize, usize),
	chars: Vec<char>,
	colors: Vec<Color>,
	escapes: Vec<Vec<usize>>, // Custom escape sequences
}

impl Pixels {
	pub fn new(size: (usize, usize)) -> Pixels {
		Pixels {
			size,
			chars: vec![' '; size.0 * size.1],
			colors: vec![Color::new_rgb(255, 255, 255); size.0 * size.1],
			escapes: vec![Vec::new(); size.0 * size.1],
		}
	}
	
	// Reset the pixels to empty and white
	pub fn clear(&mut self) {
		self.chars = vec![' '; self.size.0 * self.size.1];
		self.colors = vec![Color::new_rgb(255, 255, 255); self.size.0 * self.size.1];
		self.escapes = vec![Vec::new(); self.size.0 * self.size.1];
	}
	
	// Set the character of the pixel at the given position
	pub fn set_char(&mut self, position: (usize, usize), character: char) {
		self.chars[position.1 * self.size.0 + position.0] = character;
	}
	
	// Set the color of the pixel at the given position
	pub fn set_color(&mut self, position: (usize, usize), color: Color) {
		self.colors[position.1 * self.size.0 + position.0] = color;
	}
	
	// Set custom escape sequences for the pixel at the given position
	pub fn set_escapes(&mut self, position: (usize, usize), escapes: Vec<usize>) {
		self.escapes[position.1 * self.size.0 + position.0] = escapes;
	}
	
	// Add a custom escape sequence to the pixel at the given position
	pub fn add_escape(&mut self, position: (usize, usize), escape: usize) {
		self.escapes[position.1 * self.size.0 + position.0].push(escape);
	}
	
	// Underline the pixel at the given position
	pub fn underline(&mut self, position: (usize, usize)) {
		self.add_escape(position, 4);
	}
	
	// Bold the pixel at the given position
	pub fn bold(&mut self, position: (usize, usize)) {
		self.add_escape(position, 1);
	}
	
	// Get the color of a pixel at the given position
	pub fn get_pixel(&self, position: (usize, usize)) -> Option<(char, Color)> {
		if position.0 < self.size.0 && position.1 < self.size.1 {
			Some((self.chars[position.1 * self.size.0 + position.0], self.colors[position.1 * self.size.0 + position.0]))
		} else {
			None
		}
	}
	
	// Color all the pixels in the given color
	pub fn color_all(&mut self, color: Color) {
		for i in 0..self.size.1 {
			for j in 0..self.size.0 {
				self.set_color((j, i), color);
			}
		}
	}
	
	// Print a string to the pixels in the given color
	pub fn print(&mut self, text: &str, color: Color, underline: bool, bold: bool, position: (usize, usize), hc: HCentering, vc: VCentering) {
		let colored_text = ColoredText::new_text(text, color, underline, bold);
		self.print_color(&colored_text, position, hc, vc)
	}
	
	// Print a ColoredText to the pixels
	pub fn print_color(&mut self, text: &ColoredText, position: (usize, usize), hc: HCentering, vc: VCentering) {
		let ttext:Vec<char> = text.text().chars().collect();
		
		let mut shift = match hc {
			HCentering::Left => 0,
			HCentering::Center => - (text.split('\n').iter().max_by_key(|s| s.len()).map_or(0, |s| s.len()) as i32 / 2),
			HCentering::Right => - (text.split('\n').iter().max_by_key(|s| s.len()).map_or(0, |s| s.len()) as i32)
		};
		
		let mut y = match vc {
			VCentering::Top => position.1 as i32,
			VCentering::Middle => position.1 as i32 - {let mut count = 0; for c in text.chars() { if c == '\n' { count += 1; } } count} / 2,
			VCentering::Bottom => position.1 as i32 - {let mut count = 0; for c in text.chars() { if c == '\n' { count += 1; } } count}
		};
		
		let mut current_line_len = 0;
		
		for i in 0..text.len() {
			if ttext[i] == '\n' {
				y += 1;
				shift -= current_line_len + 1;
				current_line_len = 0;
			} else {
				let pos = position.0 as i32 + shift + i as i32;
				
				if pos >= 0 && pos < self.size.0 as i32 && y >= 0 && y < self.size.1 as i32{
					self.set_char((pos as usize, y as usize), ttext[i]);
					self.set_color((pos as usize, y as usize), text.colors()[i]);
					if text.underline()[i] {
						self.underline((pos as usize, y as usize));
					}
					if text.bold()[i] {
						self.bold((pos as usize, y as usize));
					}
				}
				
				current_line_len += 1;
			}
		}
	}
	
	// Composite to_comp onto self centered at position (0,0 = top left corner of self)
	pub fn comp(&mut self, to_comp: &Pixels, position: (i32, i32)) {
		let origin = (position.0 - to_comp.size.0  as i32 / 2, position.1 - to_comp.size.1 as i32 / 2);
		
		for i in 0..to_comp.size.1 {
			let y = origin.1 + i as i32;
			
			for j in 0..to_comp.size.0 {
				let x = origin.0 + j as i32;
				
				if x >= 0 && x < self.size.0 as i32 && y >= 0 && y < self.size.1 as i32 && to_comp.chars[i * to_comp.size.0 + j] != ' ' {
					self.set_char((x as usize, y as usize), to_comp.chars[i * to_comp.size.0 + j]);
					self.set_color((x as usize, y as usize), to_comp.colors[i * to_comp.size.0 + j]);
				}
			}
		}
	}
	
	// Render the pixels line by line
	pub fn render (&self, term_color_support: &TermColorSupport) {
		for i in 0..self.size.1 {
			crossterm::execute!(std::io::stdout(), crossterm::cursor::MoveTo(0, i as u16)).unwrap();
			let mut line = String::new();
			for j in 0..self.size.0 {
				if self.escapes[i * self.size.0 + j].len() > 0 {
					line.push_str("\x1b[");
					for e in &self.escapes[i * self.size.0 + j] {
						line.push_str(&e.to_string());
						line.push(';');
					}
					line.pop();
					line.push_str("m");
				}
				
				match self.colors[i * self.size.0 + j].to_escape(&term_color_support) {
					Some(e) => line.push_str(e.as_str()),
					None => {}
				}
				
				line.push(self.chars[i * self.size.0 + j] as char);
				
				line.push_str("\x1b[0m");
			}
			print!("{}", line);
		}
	}
}