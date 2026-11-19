use std::fmt::Write;

use serde::Serialize;

use krilla::{
    Document,
    geom::{Point, Size, Transform},
    image::Image,
    page::PageSettings,
    surface::Surface,
    text::{Font, TextDirection},
};

use super::report_graphics::{FONT, markup};

#[derive(Clone, Debug, Serialize)]
pub struct BulletinTable {
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BulletinFigure {
    pub title: String,
    pub filename: String,
    #[serde(skip_serializing)]
    pub png: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BulletinContent {
    pub title: String,
    pub details: Vec<String>,
    pub tables: Vec<BulletinTable>,
    pub figures: Vec<BulletinFigure>,
    pub notes: Vec<String>,
}

pub fn bulletin_html(content: &BulletinContent) -> String {
    let title = markup(&content.title);
    let mut html = format!(
        r#"<!doctype html><html lang="pt-BR"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{title}</title><style>body{{font:16px system-ui;color:#273d48;margin:2rem auto;padding:0 1.2rem;max-width:78rem}}table{{border-collapse:collapse;width:100%;font-size:.9rem}}th,td{{padding:.55rem;border-block-end:1px solid #dce6e9;text-align:left}}th{{background:#edf5f6}}figure{{margin:1rem 0}}img{{max-width:100%}}h2{{margin-top:2rem}}.table{{overflow:auto}}</style><main><h1>{title}</h1>"#
    );
    for line in &content.details {
        let line = markup(line);
        let _ = write!(html, "<p>{line}</p>");
    }
    for table in &content.tables {
        let title = markup(&table.title);
        let _ = write!(
            html,
            "<h2>{title}</h2><div class=\"table\"><table><thead><tr>"
        );
        for column in &table.columns {
            let cell = markup(column);
            let _ = write!(html, "<th scope=\"col\">{cell}</th>");
        }
        html.push_str("</tr></thead><tbody>");
        for row in &table.rows {
            html.push_str("<tr>");
            for value in row {
                let cell = markup(value);
                let _ = write!(html, "<td>{cell}</td>");
            }
            html.push_str("</tr>");
        }
        html.push_str("</tbody></table></div>");
    }
    for figure in &content.figures {
        let title = markup(&figure.title);
        let path = markup(&figure.filename);
        let _ = write!(
            html,
            "<figure><figcaption>{title}</figcaption><img src=\"{path}\" alt=\"{title}\"></figure>"
        );
    }
    if !content.notes.is_empty() {
        html.push_str("<h2>Notas</h2><ul>");
        for note in &content.notes {
            let note = markup(note);
            let _ = write!(html, "<li>{note}</li>");
        }
        html.push_str("</ul>");
    }
    html.push_str("</main></html>");
    html
}

fn md(text: &str) -> String {
    markup(text)
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', " ")
}

pub fn bulletin_markdown(content: &BulletinContent) -> String {
    let title = md(&content.title);
    let mut output = format!("# {title}\n");
    for line in &content.details {
        let line = md(line);
        let _ = writeln!(output, "\n{line}");
    }
    for table in &content.tables {
        let title = md(&table.title);
        let cols = table
            .columns
            .iter()
            .map(|value| md(value))
            .collect::<Vec<_>>()
            .join(" | ");
        let rules = vec!["---"; table.columns.len()].join(" | ");
        let _ = writeln!(output, "\n## {title}\n\n| {cols} |\n| {rules} |");
        for row in &table.rows {
            let row = row
                .iter()
                .map(|value| md(value))
                .collect::<Vec<_>>()
                .join(" | ");
            let _ = writeln!(output, "| {row} |");
        }
    }
    for figure in &content.figures {
        let title = md(&figure.title).replace('[', "\\[").replace(']', "\\]");
        let _ = writeln!(output, "\n## {title}\n\n![{title}]({})", figure.filename);
    }
    if !content.notes.is_empty() {
        output.push_str("\n## Notas\n\n");
        for note in &content.notes {
            let note = md(note);
            let _ = writeln!(output, "- {note}");
        }
    }
    output
}

fn text(surface: &mut Surface<'_>, font: &Font, x: f32, y: f32, size: f32, text: &str) {
    surface.draw_text(
        Point::from_xy(x, y),
        font.clone(),
        size,
        text,
        false,
        TextDirection::Auto,
    );
}

fn wrapped(text: &str, width: usize) -> Vec<String> {
    let mut lines = vec![String::new()];
    for mut word in text.split_whitespace() {
        while let Some((split, _)) = word.char_indices().nth(width) {
            if !lines.last().unwrap().is_empty() {
                lines.push(String::new());
            }
            lines.last_mut().unwrap().push_str(&word[..split]);
            lines.push(String::new());
            word = &word[split..];
        }
        let line = lines.last_mut().unwrap();
        if line.is_empty() {
            line.push_str(word);
        } else if line.chars().count() + word.chars().count() < width {
            line.push(' ');
            line.push_str(word);
        } else {
            lines.push(word.into());
        }
    }
    lines
}

fn pdf_table(
    document: &mut Document,
    font: &Font,
    page_size: Size,
    table: &BulletinTable,
    title: &str,
    indices: &[usize],
) -> Result<(), String> {
    let width = 758.0 / indices.len() as f32;
    let characters = ((width - 10.0) / 5.1).floor().max(1.0) as usize;
    let rows: Vec<_> = table
        .rows
        .iter()
        .map(|row| {
            let cells: Vec<_> = indices
                .iter()
                .map(|&i| {
                    let text = row.get(i).map(String::as_str).unwrap_or_default();
                    wrapped(text, characters)
                })
                .collect();
            let height = cells.iter().map(Vec::len).max().unwrap_or(1) as f32 * 12.0 + 7.0;
            (cells, height)
        })
        .collect();
    let mut rows = rows.as_slice();
    loop {
        let mut page = document.start_page_with(PageSettings::new(page_size));
        let mut surface = page.surface();
        text(&mut surface, font, 42.0, 50.0, 18.0, title);
        let mut header_lines = 1;
        for (c, &index) in indices.iter().enumerate() {
            let label = wrapped(&table.columns[index], characters);
            header_lines = header_lines.max(label.len());
            for (i, line) in label.iter().enumerate() {
                text(
                    &mut surface,
                    font,
                    42.0 + c as f32 * width,
                    78.0 + i as f32 * 12.0,
                    9.0,
                    line,
                );
            }
        }
        let mut y = 84.0 + header_lines as f32 * 12.0;
        while let Some(((cells, height), rest)) = rows.split_first() {
            if y + height > 557.0 {
                if y == 84.0 + header_lines as f32 * 12.0 {
                    return Err("bulletin table row exceeds one page".into());
                }
                break;
            }
            for (column, cell) in cells.iter().enumerate() {
                for (index, line) in cell.iter().enumerate() {
                    text(
                        &mut surface,
                        font,
                        42.0 + column as f32 * width,
                        y + index as f32 * 12.0,
                        9.0,
                        line,
                    );
                }
            }
            y += height;
            rows = rest;
        }
        surface.finish();
        page.finish();
        if rows.is_empty() {
            break;
        }
    }
    Ok(())
}

pub fn bulletin_pdf(content: &BulletinContent) -> Result<Vec<u8>, String> {
    let font = Font::new(FONT.to_vec().into(), 0).ok_or("invalid bulletin font")?;
    let mut document = Document::new();
    let page_size = Size::from_wh(842.0, 595.0).unwrap();
    let mut headline = Vec::new();
    headline.extend(wrapped(&content.title, 68));
    let mut details = Vec::new();
    for detail in &content.details {
        details.extend(wrapped(detail, 110));
    }
    for (index, chunk) in details.chunks(24).enumerate() {
        let mut page = document.start_page_with(PageSettings::new(page_size));
        let mut surface = page.surface();
        let mut y = 55.0;
        if index == 0 {
            for line in &headline {
                text(&mut surface, &font, 42.0, y, 20.0, line);
                y += 25.0;
            }
            y += 22.0;
        }
        for line in chunk {
            text(&mut surface, &font, 42.0, y, 12.0, line);
            y += 18.0;
        }
        surface.finish();
        page.finish();
    }

    for table in &content.tables {
        if table.columns.is_empty() {
            continue;
        }
        const OTHER_COLUMNS: usize = 11;
        let blocks = (table.columns.len() - 1).div_ceil(OTHER_COLUMNS).max(1);
        for block in 0..blocks {
            let start = 1 + block * OTHER_COLUMNS;
            let end = (start + OTHER_COLUMNS).min(table.columns.len());
            let indices: Vec<_> = std::iter::once(0).chain(start..end).collect();
            let title = if block == 0 {
                table.title.clone()
            } else {
                format!("{} · continuação", table.title)
            };
            pdf_table(&mut document, &font, page_size, table, &title, &indices)?;
        }
    }
    for figure in &content.figures {
        let mut page = document.start_page_with(PageSettings::new(page_size));
        let mut surface = page.surface();
        for (i, line) in wrapped(&figure.title, 90).iter().enumerate() {
            text(
                &mut surface,
                &font,
                42.0,
                50.0 + i as f32 * 18.0,
                16.0,
                line,
            );
        }
        let image = Image::from_png(figure.png.clone().into(), true)?;
        surface.push_transform(&Transform::from_translate(42.0, 100.0));
        let (width, height) = image.size();
        let height = 758.0 * height as f32 / width as f32;
        surface.draw_image(
            image,
            Size::from_wh(758.0, height).ok_or("invalid bulletin figure size")?,
        );
        surface.pop();
        surface.finish();
        page.finish();
    }
    if !content.notes.is_empty() {
        let notes: Vec<_> = content
            .notes
            .iter()
            .flat_map(|note| wrapped(note, 110))
            .collect();
        for chunk in notes.chunks(29) {
            let mut page = document.start_page_with(PageSettings::new(page_size));
            let mut surface = page.surface();
            text(&mut surface, &font, 42.0, 50.0, 18.0, "Notas");
            for (i, line) in chunk.iter().enumerate() {
                text(
                    &mut surface,
                    &font,
                    42.0,
                    78.0 + i as f32 * 16.0,
                    11.0,
                    line,
                );
            }
            surface.finish();
            page.finish();
        }
    }
    document.finish().map_err(|e| format!("{e:?}"))
}
