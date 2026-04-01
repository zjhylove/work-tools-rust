use anyhow::Result;
use printpdf::*;

use crate::models::*;
use super::DocumentExporter;

/// PDF document exporter
pub struct PdfExporter {
    template_style: TemplateStyle,
}

/// Page dimensions in mm
const PAGE_WIDTH: f32 = 210.0;
const PAGE_HEIGHT: f32 = 297.0;
const MARGIN_LEFT: f32 = 15.0;
const MARGIN_RIGHT: f32 = 15.0;
const MARGIN_TOP: f32 = 15.0;
const MARGIN_BOTTOM: f32 = 15.0;

/// Content area width
const CONTENT_WIDTH: f32 = PAGE_WIDTH - MARGIN_LEFT - MARGIN_RIGHT;

/// Line heights
const TITLE_FONT_SIZE: f32 = 18.0;
const HEADING_FONT_SIZE: f32 = 14.0;
const SUBHEADING_FONT_SIZE: f32 = 11.0;
const TABLE_FONT_SIZE: f32 = 9.0;
const LINE_HEIGHT: f32 = 7.0;
const TABLE_ROW_HEIGHT: f32 = 6.5;
const TABLE_HEADER_HEIGHT: f32 = 7.0;
const SECTION_GAP: f32 = 10.0;

impl PdfExporter {
    pub fn new(template_style: TemplateStyle) -> Self {
        Self { template_style }
    }

    /// Format data type with optional length
    fn format_data_type(col: &ColumnInfo) -> String {
        if let Some(len) = col.max_length {
            format!("{}({})", col.data_type.to_uppercase(), len)
        } else {
            col.data_type.to_uppercase()
        }
    }

    /// Draw text at position and return the new Y position (moved up by nothing,
    /// caller controls spacing). Y is measured from the bottom of the page.
    fn draw_text(
        layer: &PdfLayerReference,
        font: &IndirectFontRef,
        font_size: f32,
        x: Mm,
        y: Mm,
        text: &str,
    ) {
        layer.use_text(text, font_size, x, y, font);
    }

    /// Draw a horizontal line from (x1, y) to (x2, y)
    fn draw_hline(layer: &PdfLayerReference, x1: f32, y: f32, x2: f32, thickness: f32) {
        let line = Line {
            points: vec![
                (Point::new(Mm(x1), Mm(y)), false),
                (Point::new(Mm(x2), Mm(y)), false),
            ],
            is_closed: false,
        };
        layer.set_outline_thickness(thickness);
        layer.add_line(line);
    }

    /// Draw a vertical line from (x, y1) to (x, y2)
    fn draw_vline(layer: &PdfLayerReference, x: f32, y1: f32, y2: f32, thickness: f32) {
        let line = Line {
            points: vec![
                (Point::new(Mm(x), Mm(y1)), false),
                (Point::new(Mm(x), Mm(y2)), false),
            ],
            is_closed: false,
        };
        layer.set_outline_thickness(thickness);
        layer.add_line(line);
    }

    /// Draw a filled rectangle for table header background
    fn draw_rect_fill(layer: &PdfLayerReference, x: f32, y_top: f32, width: f32, height: f32) {
        let poly = Polygon {
            rings: vec![vec![
                (Point::new(Mm(x), Mm(y_top - height)), false),
                (Point::new(Mm(x + width), Mm(y_top - height)), false),
                (Point::new(Mm(x + width), Mm(y_top)), false),
                (Point::new(Mm(x), Mm(y_top)), false),
            ]],
            mode: PolygonMode::Fill,
            winding_order: WindingOrder::NonZero,
        };
        layer.add_polygon(poly);
    }

    /// Check if we need a new page. Returns (page_index, layer_index, new_y).
    fn add_new_page(
        doc: &PdfDocumentReference,
    ) -> (PdfPageIndex, PdfLayerIndex) {
        doc.add_page(Mm(PAGE_WIDTH), Mm(PAGE_HEIGHT), "Layer 1")
    }

    /// Render the title page section. Returns the current Y position.
    fn render_title(
        &self,
        doc: &PdfDocumentReference,
        layer: &PdfLayerReference,
        font: &IndirectFontRef,
        font_bold: &IndirectFontRef,
        tables: &[TableInfo],
        current_y: &mut f32,
        current_page: &mut PdfPageIndex,
        current_layer: &mut PdfLayerIndex,
    ) {
        // Title
        Self::draw_text(layer, font_bold, TITLE_FONT_SIZE, Mm(MARGIN_LEFT), Mm(*current_y), "Database Documentation");
        *current_y -= LINE_HEIGHT * 1.5;

        // Generation timestamp
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let gen_text = format!("Generated: {}", timestamp);
        Self::draw_text(layer, font, SUBHEADING_FONT_SIZE, Mm(MARGIN_LEFT), Mm(*current_y), &gen_text);
        *current_y -= LINE_HEIGHT;

        // Table of contents
        Self::draw_text(layer, font_bold, HEADING_FONT_SIZE, Mm(MARGIN_LEFT), Mm(*current_y), "Table of Contents");
        *current_y -= LINE_HEIGHT;

        for table in tables {
            if *current_y < MARGIN_BOTTOM + LINE_HEIGHT {
                let (new_page, new_layer) = Self::add_new_page(doc);
                *current_page = new_page;
                *current_layer = new_layer;
                *current_y = PAGE_HEIGHT - MARGIN_TOP;
                let new_l = doc.get_page(*current_page).get_layer(*current_layer);
                Self::draw_text(&new_l, font, TABLE_FONT_SIZE, Mm(MARGIN_LEFT), Mm(*current_y), &format!("- {}", table.name));
            } else {
                Self::draw_text(layer, font, TABLE_FONT_SIZE, Mm(MARGIN_LEFT + 5.0), Mm(*current_y), &format!("- {}", table.name));
            }
            *current_y -= LINE_HEIGHT;
        }

        *current_y -= SECTION_GAP;
    }

    /// Render a simple template table (3 columns: Name, Type, Description)
    fn render_simple_table(
        &self,
        doc: &PdfDocumentReference,
        font: &IndirectFontRef,
        font_bold: &IndirectFontRef,
        table: &TableInfo,
        current_y: &mut f32,
        current_page: &mut PdfPageIndex,
        current_layer: &mut PdfLayerIndex,
    ) {
        let col_widths = [50.0, 50.0, 80.0];
        let headers = ["Name", "Type", "Description"];
        let row_height = TABLE_ROW_HEIGHT;

        // Check if we have enough space for at least heading + header + one row
        let min_space = LINE_HEIGHT + TABLE_HEADER_HEIGHT + row_height + SECTION_GAP;
        if *current_y < MARGIN_BOTTOM + min_space {
            let (new_page, new_layer) = Self::add_new_page(doc);
            *current_page = new_page;
            *current_layer = new_layer;
            *current_y = PAGE_HEIGHT - MARGIN_TOP;
        }

        // Get current layer (may have changed)
        let layer = doc.get_page(*current_page).get_layer(*current_layer);

        // Table heading
        let heading = match &table.comment {
            Some(c) if !c.is_empty() => format!("{} ({})", table.name, c),
            _ => table.name.clone(),
        };
        Self::draw_text(&layer, font_bold, HEADING_FONT_SIZE, Mm(MARGIN_LEFT), Mm(*current_y), &heading);
        *current_y -= LINE_HEIGHT;

        // Check page break
        if *current_y < MARGIN_BOTTOM + TABLE_HEADER_HEIGHT + row_height {
            let (new_page, new_layer) = Self::add_new_page(doc);
            *current_page = new_page;
            *current_layer = new_layer;
            *current_y = PAGE_HEIGHT - MARGIN_TOP;
        }

        let layer = doc.get_page(*current_page).get_layer(*current_layer);

        // Draw header background
        Self::set_gray_fill(&layer, 0.9);
        Self::draw_rect_fill(&layer, MARGIN_LEFT, *current_y, CONTENT_WIDTH, TABLE_HEADER_HEIGHT);
        Self::set_gray_fill(&layer, 1.0); // Reset to white

        // Draw header text
        let mut x = MARGIN_LEFT + 2.0;
        for (i, header) in headers.iter().enumerate() {
            Self::draw_text(&layer, font_bold, TABLE_FONT_SIZE, Mm(x), Mm(*current_y - TABLE_HEADER_HEIGHT + 1.5), header);
            x += col_widths[i];
        }

        // Header bottom line
        Self::draw_hline(&layer, MARGIN_LEFT, *current_y - TABLE_HEADER_HEIGHT, MARGIN_LEFT + CONTENT_WIDTH, 0.5);
        *current_y -= TABLE_HEADER_HEIGHT;

        // Data rows
        for col in &table.columns {
            if *current_y < MARGIN_BOTTOM + row_height {
                let (new_page, new_layer) = Self::add_new_page(doc);
                *current_page = new_page;
                *current_layer = new_layer;
                *current_y = PAGE_HEIGHT - MARGIN_TOP;
            }

            let layer = doc.get_page(*current_page).get_layer(*current_layer);

            let comment = col.comment.as_deref().unwrap_or("-");
            let data_type = Self::format_data_type(col);
            let values = [
                col.name.as_str(),
                data_type.as_str(),
                comment,
            ];

            let mut x = MARGIN_LEFT + 2.0;
            for (i, val) in values.iter().enumerate() {
                Self::draw_text(&layer, font, TABLE_FONT_SIZE, Mm(x), Mm(*current_y - row_height + 1.5), val);
                x += col_widths[i];
            }

            // Row bottom line
            Self::draw_hline(&layer, MARGIN_LEFT, *current_y - row_height, MARGIN_LEFT + CONTENT_WIDTH, 0.3);
            *current_y -= row_height;
        }

        // Table border (top + bottom)
        let layer = doc.get_page(*current_page).get_layer(*current_layer);
        Self::draw_hline(&layer, MARGIN_LEFT, *current_y, MARGIN_LEFT + CONTENT_WIDTH, 0.5);

        // Vertical lines for columns
        let table_top = *current_y + TABLE_HEADER_HEIGHT + (table.columns.len() as f32 * row_height);
        let mut x = MARGIN_LEFT;
        for width in &col_widths {
            Self::draw_vline(&layer, x, *current_y, table_top, 0.3);
            x += width;
        }
        Self::draw_vline(&layer, MARGIN_LEFT + CONTENT_WIDTH, *current_y, table_top, 0.3);

        *current_y -= SECTION_GAP;
    }

    /// Render a detailed template table (6 columns) + index section
    fn render_detailed_table(
        &self,
        doc: &PdfDocumentReference,
        font: &IndirectFontRef,
        font_bold: &IndirectFontRef,
        table: &TableInfo,
        current_y: &mut f32,
        current_page: &mut PdfPageIndex,
        current_layer: &mut PdfLayerIndex,
    ) {
        let col_widths = [30.0, 35.0, 20.0, 20.0, 35.0, 40.0];
        let headers = ["Name", "Type", "Nullable", "PK", "Default", "Description"];
        let row_height = TABLE_ROW_HEIGHT;

        // Check if we have enough space for at least heading + header + one row
        let min_space = LINE_HEIGHT * 3.0 + TABLE_HEADER_HEIGHT + row_height + SECTION_GAP;
        if *current_y < MARGIN_BOTTOM + min_space {
            let (new_page, new_layer) = Self::add_new_page(doc);
            *current_page = new_page;
            *current_layer = new_layer;
            *current_y = PAGE_HEIGHT - MARGIN_TOP;
        }

        let layer = doc.get_page(*current_page).get_layer(*current_layer);

        // Table heading
        Self::draw_text(&layer, font_bold, HEADING_FONT_SIZE, Mm(MARGIN_LEFT), Mm(*current_y), &table.name);
        *current_y -= LINE_HEIGHT;

        // Schema info
        let schema_text = format!("Schema: {}", table.schema);
        Self::draw_text(&layer, font, TABLE_FONT_SIZE, Mm(MARGIN_LEFT), Mm(*current_y), &schema_text);
        *current_y -= LINE_HEIGHT * 0.8;

        if let Some(ref comment) = table.comment {
            if !comment.is_empty() {
                let comment_text = format!("Comment: {}", comment);
                Self::draw_text(&layer, font, TABLE_FONT_SIZE, Mm(MARGIN_LEFT), Mm(*current_y), &comment_text);
                *current_y -= LINE_HEIGHT * 0.8;
            }
        }

        // "Columns" subheading
        Self::draw_text(&layer, font_bold, SUBHEADING_FONT_SIZE, Mm(MARGIN_LEFT), Mm(*current_y), "Columns");
        *current_y -= LINE_HEIGHT;

        if *current_y < MARGIN_BOTTOM + TABLE_HEADER_HEIGHT + row_height {
            let (new_page, new_layer) = Self::add_new_page(doc);
            *current_page = new_page;
            *current_layer = new_layer;
            *current_y = PAGE_HEIGHT - MARGIN_TOP;
        }

        let layer = doc.get_page(*current_page).get_layer(*current_layer);

        // Track top of column table for vertical lines
        let col_table_start_y = *current_y;

        // Draw header background
        Self::set_gray_fill(&layer, 0.9);
        Self::draw_rect_fill(&layer, MARGIN_LEFT, *current_y, CONTENT_WIDTH, TABLE_HEADER_HEIGHT);
        Self::set_gray_fill(&layer, 1.0);

        // Draw header text
        let mut x = MARGIN_LEFT + 2.0;
        for (i, header) in headers.iter().enumerate() {
            Self::draw_text(&layer, font_bold, TABLE_FONT_SIZE, Mm(x), Mm(*current_y - TABLE_HEADER_HEIGHT + 1.5), header);
            x += col_widths[i];
        }

        // Header bottom line
        Self::draw_hline(&layer, MARGIN_LEFT, *current_y - TABLE_HEADER_HEIGHT, MARGIN_LEFT + CONTENT_WIDTH, 0.5);
        *current_y -= TABLE_HEADER_HEIGHT;

        // Data rows
        for col in &table.columns {
            if *current_y < MARGIN_BOTTOM + row_height {
                let (new_page, new_layer) = Self::add_new_page(doc);
                *current_page = new_page;
                *current_layer = new_layer;
                *current_y = PAGE_HEIGHT - MARGIN_TOP;
            }

            let layer = doc.get_page(*current_page).get_layer(*current_layer);

            let nullable = if col.is_nullable { "Y" } else { "N" };
            let pk = if col.is_primary_key { "Y" } else { "N" };
            let default = col.default_value.as_deref().unwrap_or("-");
            let comment = col.comment.as_deref().unwrap_or("-");
            let data_type = Self::format_data_type(col);

            let values = [
                col.name.as_str(),
                data_type.as_str(),
                nullable,
                pk,
                default,
                comment,
            ];

            let mut x = MARGIN_LEFT + 2.0;
            for (i, val) in values.iter().enumerate() {
                Self::draw_text(&layer, font, TABLE_FONT_SIZE, Mm(x), Mm(*current_y - row_height + 1.5), val);
                x += col_widths[i];
            }

            // Row bottom line
            Self::draw_hline(&layer, MARGIN_LEFT, *current_y - row_height, MARGIN_LEFT + CONTENT_WIDTH, 0.3);
            *current_y -= row_height;
        }

        // Table bottom border
        let layer = doc.get_page(*current_page).get_layer(*current_layer);
        Self::draw_hline(&layer, MARGIN_LEFT, *current_y, MARGIN_LEFT + CONTENT_WIDTH, 0.5);

        // Vertical lines for column table
        let col_table_end_y = *current_y;
        let mut x = MARGIN_LEFT;
        for width in &col_widths {
            Self::draw_vline(&layer, x, col_table_end_y, col_table_start_y, 0.3);
            x += width;
        }
        Self::draw_vline(&layer, MARGIN_LEFT + CONTENT_WIDTH, col_table_end_y, col_table_start_y, 0.3);

        *current_y -= SECTION_GAP;

        // Index section
        if !table.indexes.is_empty() {
            self.render_index_section(doc, font, font_bold, table, current_y, current_page, current_layer);
        }
    }

    /// Render the index section for a table
    fn render_index_section(
        &self,
        doc: &PdfDocumentReference,
        font: &IndirectFontRef,
        font_bold: &IndirectFontRef,
        table: &TableInfo,
        current_y: &mut f32,
        current_page: &mut PdfPageIndex,
        current_layer: &mut PdfLayerIndex,
    ) {
        let idx_col_widths = [50.0, 70.0, 35.0, 25.0];
        let idx_headers = ["Index", "Columns", "Unique", "Type"];
        let row_height = TABLE_ROW_HEIGHT;

        if *current_y < MARGIN_BOTTOM + LINE_HEIGHT + TABLE_HEADER_HEIGHT + row_height {
            let (new_page, new_layer) = Self::add_new_page(doc);
            *current_page = new_page;
            *current_layer = new_layer;
            *current_y = PAGE_HEIGHT - MARGIN_TOP;
        }

        let layer = doc.get_page(*current_page).get_layer(*current_layer);

        // "Indexes" subheading
        Self::draw_text(&layer, font_bold, SUBHEADING_FONT_SIZE, Mm(MARGIN_LEFT), Mm(*current_y), "Indexes");
        *current_y -= LINE_HEIGHT;

        let idx_table_start_y = *current_y;

        // Draw header background
        Self::set_gray_fill(&layer, 0.9);
        Self::draw_rect_fill(&layer, MARGIN_LEFT, *current_y, CONTENT_WIDTH, TABLE_HEADER_HEIGHT);
        Self::set_gray_fill(&layer, 1.0);

        // Draw header text
        let mut x = MARGIN_LEFT + 2.0;
        for (i, header) in idx_headers.iter().enumerate() {
            Self::draw_text(&layer, font_bold, TABLE_FONT_SIZE, Mm(x), Mm(*current_y - TABLE_HEADER_HEIGHT + 1.5), header);
            x += idx_col_widths[i];
        }

        // Header bottom line
        Self::draw_hline(&layer, MARGIN_LEFT, *current_y - TABLE_HEADER_HEIGHT, MARGIN_LEFT + CONTENT_WIDTH, 0.5);
        *current_y -= TABLE_HEADER_HEIGHT;

        // Index rows
        for idx in &table.indexes {
            if *current_y < MARGIN_BOTTOM + row_height {
                let (new_page, new_layer) = Self::add_new_page(doc);
                *current_page = new_page;
                *current_layer = new_layer;
                *current_y = PAGE_HEIGHT - MARGIN_TOP;
            }

            let layer = doc.get_page(*current_page).get_layer(*current_layer);

            let unique = if idx.is_unique { "Y" } else { "N" };
            let idx_type = if idx.is_primary { "PK" } else { "Normal" };
            let columns = idx.columns.join(", ");

            let values = [
                idx.name.as_str(),
                columns.as_str(),
                unique,
                idx_type,
            ];

            let mut x = MARGIN_LEFT + 2.0;
            for (i, val) in values.iter().enumerate() {
                Self::draw_text(&layer, font, TABLE_FONT_SIZE, Mm(x), Mm(*current_y - row_height + 1.5), val);
                x += idx_col_widths[i];
            }

            Self::draw_hline(&layer, MARGIN_LEFT, *current_y - row_height, MARGIN_LEFT + CONTENT_WIDTH, 0.3);
            *current_y -= row_height;
        }

        // Table bottom border
        let layer = doc.get_page(*current_page).get_layer(*current_layer);
        Self::draw_hline(&layer, MARGIN_LEFT, *current_y, MARGIN_LEFT + CONTENT_WIDTH, 0.5);

        // Vertical lines
        let idx_table_end_y = *current_y;
        let mut x = MARGIN_LEFT;
        for width in &idx_col_widths {
            Self::draw_vline(&layer, x, idx_table_end_y, idx_table_start_y, 0.3);
            x += width;
        }
        Self::draw_vline(&layer, MARGIN_LEFT + CONTENT_WIDTH, idx_table_end_y, idx_table_start_y, 0.3);

        *current_y -= SECTION_GAP;
    }

    /// Set fill color to grayscale
    fn set_gray_fill(layer: &PdfLayerReference, value: f32) {
        use printpdf::Color;
        layer.set_fill_color(Color::Greyscale(Greyscale::new(value, None)));
    }

    /// Generate PDF bytes for the given tables
    fn generate_pdf(&self, tables: &[TableInfo]) -> Result<Vec<u8>> {
        let (doc, page_index, layer_index) = PdfDocument::new(
            "Database Documentation",
            Mm(PAGE_WIDTH),
            Mm(PAGE_HEIGHT),
            "Layer 1",
        );

        // Add built-in fonts
        let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
        let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;

        let mut current_page = page_index;
        let mut current_layer = layer_index;
        let mut current_y = PAGE_HEIGHT - MARGIN_TOP;

        let layer = doc.get_page(current_page).get_layer(current_layer);

        // Set outline color to black for all lines
        use printpdf::Color;
        layer.set_outline_color(Color::Greyscale(Greyscale::new(0.0, None)));

        // Render title and table of contents
        self.render_title(
            &doc, &layer, &font, &font_bold, tables,
            &mut current_y, &mut current_page, &mut current_layer,
        );

        // Render each table
        for table in tables {
            let layer = doc.get_page(current_page).get_layer(current_layer);
            // Reset outline color on potentially new layer
            layer.set_outline_color(Color::Greyscale(Greyscale::new(0.0, None)));

            match self.template_style {
                TemplateStyle::Simple => {
                    self.render_simple_table(
                        &doc, &font, &font_bold, table,
                        &mut current_y, &mut current_page, &mut current_layer,
                    );
                }
                TemplateStyle::Detailed => {
                    self.render_detailed_table(
                        &doc, &font, &font_bold, table,
                        &mut current_y, &mut current_page, &mut current_layer,
                    );
                }
            }
        }

        // Save to bytes
        let bytes = doc.save_to_bytes()?;
        Ok(bytes)
    }
}

impl DocumentExporter for PdfExporter {
    fn export(&self, tables: &[TableInfo], config: &ExportConfig) -> Result<Vec<String>> {
        let output_dir = std::path::PathBuf::from(&config.output_dir);
        std::fs::create_dir_all(&output_dir)?;

        let file_name = format!("db_doc_{}.pdf", chrono::Local::now().format("%Y%m%d"));
        let file_path = output_dir.join(&file_name);

        let pdf_bytes = self.generate_pdf(tables)?;
        std::fs::write(&file_path, pdf_bytes)?;

        Ok(vec![file_path.to_string_lossy().to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exporter::DocumentExporter;
    use crate::models::IndexInfo;

    fn create_test_table() -> TableInfo {
        let mut table = TableInfo::new("users", "mydb");
        table.comment = Some("User table".to_string());
        table.columns = vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "bigint".to_string(),
                max_length: None,
                is_nullable: false,
                is_primary_key: true,
                default_value: None,
                comment: Some("User ID".to_string()),
                position: 1,
            },
            ColumnInfo {
                name: "username".to_string(),
                data_type: "varchar".to_string(),
                max_length: Some(255),
                is_nullable: false,
                is_primary_key: false,
                default_value: None,
                comment: Some("Username".to_string()),
                position: 2,
            },
            ColumnInfo {
                name: "email".to_string(),
                data_type: "varchar".to_string(),
                max_length: Some(255),
                is_nullable: true,
                is_primary_key: false,
                default_value: None,
                comment: Some("Email address".to_string()),
                position: 3,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "timestamp".to_string(),
                max_length: None,
                is_nullable: false,
                is_primary_key: false,
                default_value: Some("CURRENT_TIMESTAMP".to_string()),
                comment: Some("Creation time".to_string()),
                position: 4,
            },
        ];
        table.indexes = vec![
            IndexInfo {
                name: "idx_username".to_string(),
                columns: vec!["username".to_string()],
                is_unique: true,
                is_primary: false,
            },
            IndexInfo {
                name: "idx_email".to_string(),
                columns: vec!["email".to_string()],
                is_unique: true,
                is_primary: false,
            },
        ];
        table
    }

    fn create_export_config(output_dir: &str) -> ExportConfig {
        ExportConfig {
            connection_id: "test-conn".to_string(),
            tables: vec!["users".to_string()],
            output_dir: output_dir.to_string(),
            format: ExportFormat::Pdf,
            template: TemplateStyle::Detailed,
        }
    }

    #[test]
    fn test_format_data_type() {
        let col_with_len = ColumnInfo {
            name: "username".to_string(),
            data_type: "varchar".to_string(),
            max_length: Some(255),
            is_nullable: false,
            is_primary_key: false,
            default_value: None,
            comment: None,
            position: 1,
        };
        assert_eq!(PdfExporter::format_data_type(&col_with_len), "VARCHAR(255)");

        let col_without_len = ColumnInfo {
            name: "id".to_string(),
            data_type: "bigint".to_string(),
            max_length: None,
            is_nullable: false,
            is_primary_key: true,
            default_value: None,
            comment: None,
            position: 1,
        };
        assert_eq!(PdfExporter::format_data_type(&col_without_len), "BIGINT");
    }

    #[test]
    fn test_generate_pdf_simple() {
        let exporter = PdfExporter::new(TemplateStyle::Simple);
        let table = create_test_table();
        let bytes = exporter.generate_pdf(&[table]).unwrap();

        // PDF magic bytes
        assert!(bytes.starts_with(b"%PDF"));
        // Non-trivial size
        assert!(bytes.len() > 500);
    }

    #[test]
    fn test_generate_pdf_detailed() {
        let exporter = PdfExporter::new(TemplateStyle::Detailed);
        let table = create_test_table();
        let bytes = exporter.generate_pdf(&[table]).unwrap();

        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 500);
    }

    #[test]
    fn test_generate_pdf_multiple_tables() {
        let exporter = PdfExporter::new(TemplateStyle::Simple);
        let table1 = create_test_table();
        let mut table2 = TableInfo::new("orders", "mydb");
        table2.comment = Some("Order table".to_string());
        table2.columns = vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "bigint".to_string(),
                max_length: None,
                is_nullable: false,
                is_primary_key: true,
                default_value: None,
                comment: Some("Order ID".to_string()),
                position: 1,
            },
            ColumnInfo {
                name: "user_id".to_string(),
                data_type: "bigint".to_string(),
                max_length: None,
                is_nullable: false,
                is_primary_key: false,
                default_value: None,
                comment: Some("User ID".to_string()),
                position: 2,
            },
        ];

        let bytes = exporter.generate_pdf(&[table1, table2]).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 500);
    }

    #[test]
    fn test_export_creates_file() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let tmp_path = tmp_dir.path().to_string_lossy().to_string();

        let exporter = PdfExporter::new(TemplateStyle::Simple);
        let table = create_test_table();
        let config = create_export_config(&tmp_path);

        let files = exporter.export(&[table], &config).unwrap();

        assert_eq!(files.len(), 1);
        let file_path = &files[0];
        assert!(file_path.ends_with(".pdf"));

        // Verify the file exists and is non-trivial
        assert!(std::path::Path::new(file_path).exists());
        let metadata = std::fs::metadata(file_path).unwrap();
        assert!(metadata.len() > 500);

        // Verify PDF header
        let bytes = std::fs::read(file_path).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_export_detailed_creates_file() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let tmp_path = tmp_dir.path().to_string_lossy().to_string();

        let exporter = PdfExporter::new(TemplateStyle::Detailed);
        let table = create_test_table();
        let mut config = create_export_config(&tmp_path);
        config.template = TemplateStyle::Detailed;

        let files = exporter.export(&[table], &config).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with(".pdf"));
        assert!(std::path::Path::new(&files[0]).exists());

        let metadata = std::fs::metadata(&files[0]).unwrap();
        assert!(metadata.len() > 500);
    }
}
