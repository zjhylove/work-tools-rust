use anyhow::Result;
use docx_rs::*;
use std::fs::{self, File};
use std::path::Path;

use crate::exporter::DocumentExporter;
use crate::exporter::sanitize_filename;
use crate::models::ApiInfo;

const HEADER_BG: &str = "8EAADB";
const HEADER_TEXT: &str = "FFFFFF";
const CODE_BG: &str = "F5F5F5";

pub struct WordExporter;

impl DocumentExporter for WordExporter {
    fn export(
        &self,
        apis: &[ApiInfo],
        output_dir: &str,
        service_name: &str,
    ) -> Result<Vec<String>> {
        let dir = Path::new(output_dir);
        fs::create_dir_all(dir)?;

        let filename = format!("{}-api-doc.docx", sanitize_filename(service_name));
        let filepath = dir.join(&filename);
        let file = File::create(&filepath)?;

        let mut docx = Docx::new();

        // 标题
        docx = docx.add_paragraph(
            Paragraph::new().align(AlignmentType::Center).add_run(
                Run::new()
                    .add_text(format!("{} API 接口文档", service_name))
                    .bold()
                    .size(36)
                    .fonts(chinese_fonts()),
            ),
        );

        // 生成时间
        docx = docx.add_paragraph(
            Paragraph::new().align(AlignmentType::Center).add_run(
                Run::new()
                    .add_text(format!(
                        "生成时间: {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                    ))
                    .size(20)
                    .color("888888")
                    .fonts(chinese_fonts()),
            ),
        );

        // 目录
        docx = docx.add_paragraph(
            Paragraph::new().add_run(
                Run::new()
                    .add_text("目录")
                    .bold()
                    .size(28)
                    .fonts(chinese_fonts()),
            ),
        );

        for (i, api) in apis.iter().enumerate() {
            docx = docx.add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text(format!(
                            "{}. {} [{}] {}",
                            i + 1,
                            api.api_name,
                            api.http_method,
                            api.full_path
                        ))
                        .size(22)
                        .fonts(chinese_fonts()),
                ),
            );
        }

        // 每个 API 的详情
        for (i, api) in apis.iter().enumerate() {
            // 分隔线（粗段落间距代替）
            docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text("").size(12)));

            // API 标题
            docx = docx.add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text(format!(
                            "{}. {}",
                            i + 1,
                            if api.api_name.is_empty() {
                                format!("{} {}", api.http_method, api.full_path)
                            } else {
                                api.api_name.clone()
                            }
                        ))
                        .bold()
                        .size(28)
                        .fonts(chinese_fonts()),
                ),
            );

            // 接口基本信息
            docx = add_info_table(
                docx,
                &[
                    ("HTTP 方法", &api.http_method),
                    ("请求路径", &api.full_path),
                    ("服务名称", &api.service_name),
                    ("业务模块", &api.business_module),
                    ("版本", &api.version),
                ],
            );

            // 请求参数表
            if !api.req_fields.is_empty() {
                docx = docx.add_paragraph(
                    Paragraph::new().add_run(
                        Run::new()
                            .add_text("请求参数")
                            .bold()
                            .size(24)
                            .fonts(chinese_fonts()),
                    ),
                );

                docx = add_field_table(docx, &api.req_fields, true);
            }

            // 请求示例
            if !api.req_example.is_empty() {
                docx = add_code_block(docx, "请求示例", &api.req_example);
            }

            // 响应参数表
            if !api.resp_nodes.is_empty() {
                docx = docx.add_paragraph(
                    Paragraph::new().add_run(
                        Run::new()
                            .add_text("响应参数")
                            .bold()
                            .size(24)
                            .fonts(chinese_fonts()),
                    ),
                );

                for node in &api.resp_nodes {
                    if !node.node_desc.is_empty() {
                        docx = docx.add_paragraph(
                            Paragraph::new().add_run(
                                Run::new()
                                    .add_text(format!("{} ({})", node.node_name, node.node_desc))
                                    .bold()
                                    .size(22)
                                    .fonts(chinese_fonts()),
                            ),
                        );
                    } else {
                        docx = docx.add_paragraph(
                            Paragraph::new().add_run(
                                Run::new()
                                    .add_text(&node.node_name)
                                    .bold()
                                    .size(22)
                                    .fonts(chinese_fonts()),
                            ),
                        );
                    }

                    if !node.resp_fields.is_empty() {
                        docx = add_field_table(docx, &node.resp_fields, false);
                    }
                }
            }

            // 响应示例
            if !api.resp_example.is_empty() {
                docx = add_code_block(docx, "响应示例", &api.resp_example);
            }
        }

        docx.build().pack(file)?;
        Ok(vec![filepath.to_string_lossy().to_string()])
    }
}

fn chinese_fonts() -> RunFonts {
    RunFonts::new()
        .ascii("Arial")
        .hi_ansi("Arial")
        .east_asia("SimSun")
}

fn code_fonts() -> RunFonts {
    RunFonts::new()
        .ascii("Consolas")
        .hi_ansi("Consolas")
        .east_asia("Consolas")
}

fn add_info_table(docx: Docx, rows: &[(&str, &str)]) -> Docx {
    let mut table_rows = Vec::new();

    for (label, value) in rows {
        if value.is_empty() {
            continue;
        }
        table_rows.push(TableRow::new(vec![
            TableCell::new()
                .shading(Shading::new().fill(HEADER_BG).shd_type(ShdType::Clear))
                .add_paragraph(
                    Paragraph::new().add_run(
                        Run::new()
                            .add_text(label.to_string())
                            .bold()
                            .size(20)
                            .color(HEADER_TEXT)
                            .fonts(chinese_fonts()),
                    ),
                ),
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text(value.to_string())
                        .size(20)
                        .fonts(chinese_fonts()),
                ),
            ),
        ]));
    }

    if table_rows.is_empty() {
        return docx;
    }

    docx.add_table(Table::new(table_rows).set_grid(vec![2500, 6500]))
}

fn add_field_table(
    mut docx: Docx,
    fields: &[crate::models::ApiField],
    show_required: bool,
) -> Docx {
    let grid = if show_required {
        vec![2000, 1500, 800, 4700]
    } else {
        vec![2000, 1500, 5500]
    };

    let mut header_cells = vec![
        TableCell::new()
            .shading(Shading::new().fill(HEADER_BG).shd_type(ShdType::Clear))
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("字段名")
                        .bold()
                        .size(20)
                        .color(HEADER_TEXT)
                        .fonts(chinese_fonts()),
                ),
            ),
        TableCell::new()
            .shading(Shading::new().fill(HEADER_BG).shd_type(ShdType::Clear))
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("类型")
                        .bold()
                        .size(20)
                        .color(HEADER_TEXT)
                        .fonts(chinese_fonts()),
                ),
            ),
    ];

    if show_required {
        header_cells.push(
            TableCell::new()
                .shading(Shading::new().fill(HEADER_BG).shd_type(ShdType::Clear))
                .add_paragraph(
                    Paragraph::new().add_run(
                        Run::new()
                            .add_text("必填")
                            .bold()
                            .size(20)
                            .color(HEADER_TEXT)
                            .fonts(chinese_fonts()),
                    ),
                ),
        );
    }

    header_cells.push(
        TableCell::new()
            .shading(Shading::new().fill(HEADER_BG).shd_type(ShdType::Clear))
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("注释")
                        .bold()
                        .size(20)
                        .color(HEADER_TEXT)
                        .fonts(chinese_fonts()),
                ),
            ),
    );

    let mut rows = vec![TableRow::new(header_cells)];

    for field in fields {
        let mut cells = vec![
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text(field.field_name.clone())
                        .size(20)
                        .fonts(code_fonts()),
                ),
            ),
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text(field.field_type.clone())
                        .size(20)
                        .fonts(chinese_fonts()),
                ),
            ),
        ];

        if show_required {
            cells.push(
                TableCell::new().add_paragraph(
                    Paragraph::new().add_run(
                        Run::new()
                            .add_text(field.required.clone())
                            .size(20)
                            .fonts(chinese_fonts()),
                    ),
                ),
            );
        }

        cells.push(
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text(if field.comment.is_empty() {
                            field.example_value.clone()
                        } else {
                            field.comment.clone()
                        })
                        .size(20)
                        .fonts(chinese_fonts()),
                ),
            ),
        );

        rows.push(TableRow::new(cells));
    }

    docx = docx.add_table(Table::new(rows).set_grid(grid));
    docx
}

fn add_code_block(mut docx: Docx, title: &str, code: &str) -> Docx {
    docx = docx.add_paragraph(
        Paragraph::new().add_run(
            Run::new()
                .add_text(title.to_string())
                .bold()
                .size(24)
                .fonts(chinese_fonts()),
        ),
    );

    // 代码块放在灰色背景的表格单元格中
    let code_cell = TableCell::new()
        .shading(Shading::new().fill(CODE_BG).shd_type(ShdType::Clear))
        .add_paragraph(
            Paragraph::new().add_run(
                Run::new()
                    .add_text(code.to_string())
                    .size(18)
                    .fonts(code_fonts())
                    .color("333333"),
            ),
        );

    docx = docx.add_table(Table::new(vec![TableRow::new(vec![code_cell])]).set_grid(vec![9000]));

    docx
}
