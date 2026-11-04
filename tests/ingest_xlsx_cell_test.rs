// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Cells that calamine 0.30 read wrongly, read as the workbook says.
//!
//! Each workbook is assembled here from its XML parts, so the input behind
//! every assertion is on the page. The archive is stored, not deflated: the
//! reader does not care, and the test needs no zip library.
use open_ontologies::ingest::DataIngester;

const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PKG: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

/// A zip archive of `parts`, each stored uncompressed.
fn stored_zip(parts: &[(&str, String)]) -> Vec<u8> {
    let (mut out, mut directory) = (Vec::new(), Vec::new());
    for (name, body) in parts {
        let (name, body) = (name.as_bytes(), body.as_bytes());
        let offset = out.len() as u32;
        // Version needed, flags, method (stored), time, date (1980-01-01),
        // CRC-32, compressed and uncompressed size, name length, extra length.
        let mut fields = Vec::new();
        for half in [20u16, 0, 0, 0, 0x21] {
            fields.extend_from_slice(&half.to_le_bytes());
        }
        for word in [crc32(body), body.len() as u32, body.len() as u32] {
            fields.extend_from_slice(&word.to_le_bytes());
        }
        for half in [name.len() as u16, 0] {
            fields.extend_from_slice(&half.to_le_bytes());
        }
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&fields);
        out.extend_from_slice(name);
        out.extend_from_slice(body);
        // Version made by, the same fields, then comment length, disk number,
        // internal and external attributes, and where the local header starts.
        directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        directory.extend_from_slice(&20u16.to_le_bytes());
        directory.extend_from_slice(&fields);
        directory.extend_from_slice(&[0; 10]);
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name);
    }
    // End of central directory: disk numbers, the entry count for this disk
    // and in all, the directory's size and offset, and no comment.
    let (start, size, count) = (out.len() as u32, directory.len() as u32, parts.len() as u16);
    out.extend_from_slice(&directory);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    for half in [count, count] {
        out.extend_from_slice(&half.to_le_bytes());
    }
    for word in [size, start] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&[0; 2]);
    out
}

/// A one-sheet workbook. `rows` is the content of `<sheetData>`, `shared`
/// the `<si>` elements of the shared string table, and `formats` the
/// `numFmtId` of each cell style, where style `s="i"` is `formats[i]`.
/// `custom` declares number formats as (id, formatCode attribute text).
fn workbook(
    dir: &tempfile::TempDir,
    rows: &str,
    shared: &[&str],
    formats: &[u32],
    custom: &[(u32, &str)],
) -> String {
    let num_fmts: String = custom
        .iter()
        .map(|(id, code)| format!(r#"<numFmt numFmtId="{id}" formatCode="{code}"/>"#))
        .collect();
    let xfs: String = formats
        .iter()
        .map(|id| format!(r#"<xf numFmtId="{id}" xfId="0" applyNumberFormat="1"/>"#))
        .collect();
    let parts = [
        (
            "[Content_Types].xml",
            concat!(
                r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
                r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
                r#"<Default Extension="xml" ContentType="application/xml"/>"#,
                r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
                r#"</Types>"#
            )
            .to_string(),
        ),
        (
            "_rels/.rels",
            format!(
                r#"<Relationships xmlns="{PKG}"><Relationship Id="rId1" Type="{REL}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#
            ),
        ),
        (
            "xl/workbook.xml",
            format!(
                r#"<workbook xmlns="{MAIN}" xmlns:r="{REL}"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#
            ),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            format!(
                r#"<Relationships xmlns="{PKG}"><Relationship Id="rId1" Type="{REL}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{REL}/styles" Target="styles.xml"/><Relationship Id="rId3" Type="{REL}/sharedStrings" Target="sharedStrings.xml"/></Relationships>"#
            ),
        ),
        (
            "xl/worksheets/sheet1.xml",
            format!(r#"<worksheet xmlns="{MAIN}"><sheetData>{rows}</sheetData></worksheet>"#),
        ),
        (
            "xl/styles.xml",
            format!(
                r#"<styleSheet xmlns="{MAIN}"><numFmts count="{}">{num_fmts}</numFmts><cellXfs count="{}">{xfs}</cellXfs></styleSheet>"#,
                custom.len(),
                formats.len()
            ),
        ),
        (
            "xl/sharedStrings.xml",
            format!(r#"<sst xmlns="{MAIN}">{}</sst>"#, shared.concat()),
        ),
    ];
    let path = dir.path().join("book.xlsx");
    std::fs::write(&path, stored_zip(&parts)).unwrap();
    path.to_str().unwrap().to_owned()
}

/// A header row `id`, `value` and one data row per cell, `r1`, `r2`, ...
fn column(cells: &[&str]) -> String {
    let mut rows = String::from(
        r#"<row r="1"><c r="A1" t="inlineStr"><is><t>id</t></is></c><c r="B1" t="inlineStr"><is><t>value</t></is></c></row>"#,
    );
    for (i, cell) in cells.iter().enumerate() {
        let n = i + 2;
        rows += &format!(
            r#"<row r="{n}"><c r="A{n}" t="inlineStr"><is><t>r{}</t></is></c>{}</row>"#,
            i + 1,
            cell.replace("<c ", &format!(r#"<c r="B{n}" "#))
        );
    }
    rows
}

fn values(path: &str) -> Vec<String> {
    DataIngester::parse_xlsx_file(path)
        .unwrap()
        .into_iter()
        .map(|row| row["value"].clone())
        .collect()
}

#[test]
fn an_empty_shared_string_keeps_every_later_string_on_its_own_cell() {
    // `<si/>` is a shared string with no text. calamine 0.30 skipped it while
    // reading the table, so every later index read the string after its own,
    // and the last one ran off the end of the table and panicked.
    let dir = tempfile::tempdir().unwrap();
    let path = workbook(
        &dir,
        &column(&[
            r#"<c t="s"><v>0</v></c>"#,
            r#"<c t="s"><v>1</v></c>"#,
            r#"<c t="s"><v>2</v></c>"#,
            r#"<c t="s"><v>3</v></c>"#,
        ]),
        &[
            "<si><t>zero</t></si>",
            "<si/>",
            "<si><t>two</t></si>",
            "<si><t>three</t></si>",
        ],
        &[0],
        &[],
    );
    assert_eq!(values(&path), ["zero", "", "two", "three"]);
}

#[test]
fn a_shared_string_cell_with_no_index_is_empty() {
    // calamine 0.30 read an empty `<v/>` as index 0, the first string.
    let dir = tempfile::tempdir().unwrap();
    let path = workbook(
        &dir,
        &column(&[r#"<c t="s"><v/></c>"#, r#"<c t="s"><v>0</v></c>"#]),
        &["<si><t>first</t></si>"],
        &[0],
        &[],
    );
    assert_eq!(values(&path), ["", "first"]);
}

#[test]
fn a_boolean_cell_reads_its_xml_schema_value_and_refuses_any_other() {
    // calamine 0.30 read every value but "0" as true, FALSE and "" included.
    let dir = tempfile::tempdir().unwrap();
    let path = workbook(
        &dir,
        &column(&[
            r#"<c t="b"><v>1</v></c>"#,
            r#"<c t="b"><v>0</v></c>"#,
            r#"<c t="b"><v>true</v></c>"#,
            r#"<c t="b"><v>FALSE</v></c>"#,
            r#"<c t="b"><v/></c>"#,
        ]),
        &[],
        &[0],
        &[],
    );
    assert_eq!(values(&path), ["true", "false", "true", "false", ""]);

    let dir = tempfile::tempdir().unwrap();
    let path = workbook(&dir, &column(&[r#"<c t="b"><v>2</v></c>"#]), &[], &[0], &[]);
    let error = DataIngester::parse_xlsx_file(&path).unwrap_err();
    assert!(format!("{error:#}").contains("boolean"), "{error:#}");
}

#[test]
fn excel_character_escapes_are_decoded_in_every_kind_of_string() {
    // Excel writes a carriage return in a cell as `_x000D_`, a control
    // character XML cannot carry as `_x0001_` and so on, and a literal
    // `_x000D_` as `_x005F_x000D_`. calamine 0.30 returned all of them as
    // written, in shared, inline and formula strings alike.
    let dir = tempfile::tempdir().unwrap();
    let path = workbook(
        &dir,
        &column(&[
            r#"<c t="s"><v>0</v></c>"#,
            r#"<c t="inlineStr"><is><t>c_x0001_d</t></is></c>"#,
            r#"<c t="str"><f>"e"&amp;CHAR(13)&amp;"f"</f><v>e_x000D_f</v></c>"#,
            r#"<c t="s"><v>1</v></c>"#,
        ]),
        &["<si><t>a_x000D_b</t></si>", "<si><t>_x005F_x000D_</t></si>"],
        &[0],
        &[],
    );
    assert_eq!(values(&path), ["a\rb", "c\u{1}d", "e\rf", "_x000D_"]);
}

#[test]
fn east_asian_and_quoted_literal_date_formats_read_as_dates() {
    // Style 1 is built-in format 31, the ja-JP long date yyyy"年"m"月"d"日",
    // one of the East Asian built-ins (27 to 36 and 50 to 58) that calamine
    // 0.30 read as plain numbers. Style 2 is a custom date format that opens
    // with a quoted literal: Excel writes the quotes as `&quot;`, and calamine
    // 0.30 tested the format before decoding them. Style 3 is a number format
    // with a quoted literal, which stays a number, and style 0 is General.
    let dir = tempfile::tempdir().unwrap();
    let path = workbook(
        &dir,
        &column(&[
            r#"<c s="1"><v>46301</v></c>"#,
            r#"<c s="2"><v>46301.5</v></c>"#,
            r#"<c s="3"><v>46301</v></c>"#,
            r#"<c s="0"><v>46301</v></c>"#,
        ]),
        &[],
        &[0, 31, 164, 165],
        &[
            (164, "&quot;Week of &quot;d mmm yyyy"),
            (165, "&quot;No. &quot;0"),
        ],
    );
    assert_eq!(
        values(&path),
        [
            "2026-10-06T00:00:00",
            "2026-10-06T12:00:00",
            "46301",
            "46301"
        ]
    );
}
