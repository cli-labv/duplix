use console::{measure_text_width, Term};

#[derive(Clone, Copy, PartialEq)]
pub enum Align {
    Left,
    Right,
}

pub struct Table {
    headers: Vec<String>,
    aligns: Vec<Align>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(headers: &[&str], aligns: &[Align]) -> Self {
        Self {
            headers: headers.iter().map(|h| h.to_string()).collect(),
            aligns: aligns.to_vec(),
            rows: Vec::new(),
        }
    }

    pub fn row(&mut self, cells: &[String]) -> &mut Self {
        self.rows.push(cells.to_vec());
        self
    }

    fn widths(&self) -> Vec<usize> {
        let cols = self.headers.len();
        let mut w: Vec<usize> = self.headers.iter().map(|h| measure_text_width(h)).collect();
        for row in &self.rows {
            for (i, c) in row.iter().enumerate().take(cols) {
                w[i] = w[i].max(measure_text_width(c));
            }
        }
        w
    }

    fn fit(&self, widths: &mut [usize]) {
        let term = Term::stdout();
        // Solo ajustamos en una terminal real; en pipes/logs dejamos la tabla
        // completa (nada de truncar por un ancho desconocido).
        if !term.is_term() {
            return;
        }
        let (max, _) = term.size();
        let max = max as usize;
        if max <= 20 {
            return;
        }
        let min: Vec<usize> = self.headers.iter().map(|h| measure_text_width(h)).collect();
        // ancho total = bordes + padding(2 por columna)
        let overhead = widths.len() + 1 + widths.len() * 2;
        let mut total: usize = widths.iter().sum::<usize>() + overhead;
        while total > max {
            let best = widths
                .iter()
                .enumerate()
                .filter(|(i, &v)| v > min[*i])
                .max_by_key(|(_, &v)| v)
                .map(|(i, _)| i);
            match best {
                Some(idx) => {
                    widths[idx] -= 1;
                    total -= 1;
                }
                None => break,
            }
        }
    }

    fn line(&self, left: &str, mid: &str, right: &str, widths: &[usize]) -> String {
        let mut s = String::from(left);
        for (i, w) in widths.iter().enumerate() {
            s.push_str(&"─".repeat(w + 2));
            s.push_str(if i + 1 == widths.len() { right } else { mid });
        }
        s
    }

    fn cell(text: &str, width: usize, align: Align) -> String {
        let text = clip(text, width);
        let pad = width.saturating_sub(measure_text_width(&text));
        match align {
            Align::Left => format!(" {text}{} ", " ".repeat(pad)),
            Align::Right => format!(" {}{text} ", " ".repeat(pad)),
        }
    }

    pub fn render(&self) -> String {
        let mut widths = self.widths();
        self.fit(&mut widths);

        let mut out = String::new();
        out.push_str(&self.line("┌", "┬", "┐", &widths));
        out.push('\n');
        out.push('│');
        for (i, h) in self.headers.iter().enumerate() {
            out.push_str(&Self::cell(h, widths[i], Align::Left));
            out.push('│');
        }
        out.push('\n');
        out.push_str(&self.line("├", "┼", "┤", &widths));
        out.push('\n');
        for row in &self.rows {
            out.push('│');
            for (i, width) in widths.iter().enumerate() {
                let cell = row.get(i).map(String::as_str).unwrap_or("");
                let align = self.aligns.get(i).copied().unwrap_or(Align::Left);
                out.push_str(&Self::cell(cell, *width, align));
                out.push('│');
            }
            out.push('\n');
        }
        out.push_str(&self.line("└", "┴", "┘", &widths));
        out
    }

    pub fn print(&self) {
        println!("{}", self.render());
    }
}

fn clip(text: &str, width: usize) -> String {
    if measure_text_width(text) <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut w = 0;
    for ch in text.chars() {
        let cw = measure_text_width(&ch.to_string());
        if w + cw > width - 1 {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}
