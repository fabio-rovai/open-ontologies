use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;

/// Data ingester that parses structured data files into rows of key-value pairs.
pub struct DataIngester;

/// Convert a serde_json::Value to a flat string representation.
fn json_value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

impl DataIngester {
    /// Detect the file format from the file extension.
    /// Returns one of: "csv", "json", "ndjson", "xml", "yaml", "xlsx", "parquet".
    /// Defaults to "csv" if the extension is unrecognised.
    pub fn detect_format(path: &str) -> &'static str {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "csv" => "csv",
            "json" => "json",
            "jsonl" | "ndjson" => "ndjson",
            "xml" => "xml",
            "yaml" | "yml" => "yaml",
            "xlsx" => "xlsx",
            "parquet" => "parquet",
            _ => "csv",
        }
    }

    /// Parse CSV content with headers into rows.
    pub fn parse_csv(content: &str) -> Result<Vec<HashMap<String, String>>> {
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        let headers: Vec<String> = reader
            .headers()
            .context("Failed to read CSV headers")?
            .iter()
            .map(|h| h.to_string())
            .collect();

        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result.context("Failed to read CSV record")?;
            let mut row = HashMap::new();
            for (i, value) in record.iter().enumerate() {
                if let Some(key) = headers.get(i) {
                    row.insert(key.clone(), value.to_string());
                }
            }
            rows.push(row);
        }
        Ok(rows)
    }

    /// Parse a JSON string. Accepts a JSON array of objects or a single object.
    pub fn parse_json(content: &str) -> Result<Vec<HashMap<String, String>>> {
        let value: serde_json::Value =
            serde_json::from_str(content).context("Failed to parse JSON")?;
        match value {
            serde_json::Value::Array(arr) => {
                let mut rows = Vec::new();
                for item in &arr {
                    if let serde_json::Value::Object(map) = item {
                        let row: HashMap<String, String> = map
                            .iter()
                            .map(|(k, v)| (k.clone(), json_value_to_string(v)))
                            .collect();
                        rows.push(row);
                    }
                }
                Ok(rows)
            }
            serde_json::Value::Object(map) => {
                let row: HashMap<String, String> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), json_value_to_string(v)))
                    .collect();
                Ok(vec![row])
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Parse newline-delimited JSON (NDJSON / JSON Lines).
    pub fn parse_ndjson(content: &str) -> Result<Vec<HashMap<String, String>>> {
        let mut rows = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_str(trimmed).context("Failed to parse NDJSON line")?;
            if let serde_json::Value::Object(map) = value {
                let row: HashMap<String, String> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), json_value_to_string(v)))
                    .collect();
                rows.push(row);
            }
        }
        Ok(rows)
    }

    /// Parse a YAML string containing an array of objects.
    pub fn parse_yaml(content: &str) -> Result<Vec<HashMap<String, String>>> {
        let value: serde_yaml::Value =
            serde_yaml::from_str(content).context("Failed to parse YAML")?;
        match value {
            serde_yaml::Value::Sequence(seq) => {
                let mut rows = Vec::new();
                for item in &seq {
                    if let serde_yaml::Value::Mapping(map) = item {
                        let mut row = HashMap::new();
                        for (k, v) in map {
                            let key = match k {
                                serde_yaml::Value::String(s) => s.clone(),
                                other => format!("{other:?}"),
                            };
                            let val = match v {
                                serde_yaml::Value::String(s) => s.clone(),
                                serde_yaml::Value::Number(n) => n.to_string(),
                                serde_yaml::Value::Bool(b) => b.to_string(),
                                serde_yaml::Value::Null => String::new(),
                                other => format!("{other:?}"),
                            };
                            row.insert(key, val);
                        }
                        rows.push(row);
                    }
                }
                Ok(rows)
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Parse XML with a `<root><record>...</record></root>` structure.
    /// Depth 1 = root element, depth 2 = record boundary, depth 3 = field elements.
    pub fn parse_xml(content: &str) -> Result<Vec<HashMap<String, String>>> {
        use quick_xml::events::Event;
        use quick_xml::reader::Reader;

        let mut reader = Reader::from_str(content);
        reader.config_mut().trim_text(true);

        let mut rows: Vec<HashMap<String, String>> = Vec::new();
        let mut current_row: Option<HashMap<String, String>> = None;
        let mut current_field: Option<String> = None;
        let mut depth: u32 = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    depth += 1;
                    match depth {
                        1 => {
                            // Root element — nothing to do
                        }
                        2 => {
                            // Record start
                            current_row = Some(HashMap::new());
                        }
                        3 => {
                            // Field element start
                            let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                            current_field = Some(name);
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    if depth == 3 {
                        if let (Some(row), Some(field)) =
                            (&mut current_row, &current_field)
                        {
                            let text = e.unescape().unwrap_or_default().to_string();
                            row.insert(field.clone(), text);
                        }
                    }
                }
                Ok(Event::End(_)) => {
                    match depth {
                        2 => {
                            // Record end — push row
                            if let Some(row) = current_row.take() {
                                rows.push(row);
                            }
                        }
                        3 => {
                            current_field = None;
                        }
                        _ => {}
                    }
                    depth -= 1;
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(anyhow::anyhow!("XML parse error: {e}")),
                _ => {}
            }
        }
        Ok(rows)
    }

    /// Parse an Excel (.xlsx) file. First row is treated as headers.
    pub fn parse_xlsx_file(path: &str) -> Result<Vec<HashMap<String, String>>> {
        use calamine::{open_workbook, Reader, Xlsx};

        let mut workbook: Xlsx<_> =
            open_workbook(path).context("Failed to open XLSX workbook")?;

        let sheet_names = workbook.sheet_names().to_vec();
        let first_sheet = sheet_names
            .first()
            .context("XLSX workbook has no sheets")?
            .clone();

        let range = workbook
            .worksheet_range(&first_sheet)
            .context("Failed to read XLSX worksheet")?;

        let mut row_iter = range.rows();
        let header_row = row_iter.next().context("XLSX file has no rows")?;

        let headers: Vec<String> = header_row
            .iter()
            .map(|cell| Self::calamine_cell_to_string(cell))
            .collect();

        let mut rows = Vec::new();
        for data_row in row_iter {
            let mut row = HashMap::new();
            for (i, cell) in data_row.iter().enumerate() {
                if let Some(key) = headers.get(i) {
                    row.insert(key.clone(), Self::calamine_cell_to_string(cell));
                }
            }
            rows.push(row);
        }
        Ok(rows)
    }

    /// Convert a calamine Data cell to a string.
    fn calamine_cell_to_string(cell: &calamine::Data) -> String {
        match cell {
            calamine::Data::Int(i) => i.to_string(),
            calamine::Data::Float(f) => f.to_string(),
            calamine::Data::String(s) => s.clone(),
            calamine::Data::Bool(b) => b.to_string(),
            calamine::Data::DateTime(dt) => dt.to_string(),
            calamine::Data::DateTimeIso(s) => s.clone(),
            calamine::Data::DurationIso(s) => s.clone(),
            calamine::Data::Error(e) => format!("{e:?}"),
            calamine::Data::Empty => String::new(),
        }
    }

    /// Parse a Parquet file into rows.
    pub fn parse_parquet_file(path: &str) -> Result<Vec<HashMap<String, String>>> {
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        use std::fs::File;

        let file = File::open(path).context("Failed to open Parquet file")?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .context("Failed to create Parquet reader builder")?;

        let reader = builder.build().context("Failed to build Parquet reader")?;

        let mut rows = Vec::new();
        for batch_result in reader {
            let batch: arrow::array::RecordBatch =
                batch_result.context("Failed to read Parquet record batch")?;
            let schema = batch.schema();
            let num_rows = batch.num_rows();
            let num_cols = batch.num_columns();

            for row_idx in 0..num_rows {
                let mut row = HashMap::new();
                for col_idx in 0..num_cols {
                    let field = schema.field(col_idx);
                    let col = batch.column(col_idx);
                    let value = if col.is_null(row_idx) {
                        String::new()
                    } else {
                        Self::arrow_array_value_to_string(col.as_ref(), row_idx, field.data_type())
                    };
                    row.insert(field.name().clone(), value);
                }
                rows.push(row);
            }
        }
        Ok(rows)
    }

    /// Convert an Arrow array value at a given index to a string.
    fn arrow_array_value_to_string(
        array: &dyn arrow::array::Array,
        idx: usize,
        data_type: &arrow::datatypes::DataType,
    ) -> String {
        use arrow::array::*;
        use arrow::datatypes::DataType as ArrowType;

        match data_type {
            ArrowType::Boolean => {
                let a = array.as_any().downcast_ref::<BooleanArray>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Int8 => {
                let a = array.as_any().downcast_ref::<Int8Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Int16 => {
                let a = array.as_any().downcast_ref::<Int16Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Int32 => {
                let a = array.as_any().downcast_ref::<Int32Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Int64 => {
                let a = array.as_any().downcast_ref::<Int64Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::UInt8 => {
                let a = array.as_any().downcast_ref::<UInt8Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::UInt16 => {
                let a = array.as_any().downcast_ref::<UInt16Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::UInt32 => {
                let a = array.as_any().downcast_ref::<UInt32Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::UInt64 => {
                let a = array.as_any().downcast_ref::<UInt64Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Float32 => {
                let a = array.as_any().downcast_ref::<Float32Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Float64 => {
                let a = array.as_any().downcast_ref::<Float64Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Utf8 => {
                let a = array.as_any().downcast_ref::<StringArray>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::LargeUtf8 => {
                let a = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
                a.value(idx).to_string()
            }
            _ => format!("{array:?}[{idx}]"),
        }
    }

    /// Dispatch to the correct parser based on detected format.
    /// For text formats, reads the file content first.
    pub fn parse_file(path: &str) -> Result<Vec<HashMap<String, String>>> {
        let format = Self::detect_format(path);
        match format {
            "csv" => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_csv(&content)
            }
            "json" => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_json(&content)
            }
            "ndjson" => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_ndjson(&content)
            }
            "yaml" => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_yaml(&content)
            }
            "xml" => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_xml(&content)
            }
            "xlsx" => Self::parse_xlsx_file(path),
            "parquet" => Self::parse_parquet_file(path),
            _ => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_csv(&content)
            }
        }
    }

    /// Collect unique keys from all rows, sorted alphabetically.
    pub fn extract_headers(rows: &[HashMap<String, String>]) -> Vec<String> {
        let mut keys: Vec<String> = rows
            .iter()
            .flat_map(|row| row.keys().cloned())
            .collect::<std::collections::HashSet<String>>()
            .into_iter()
            .collect();
        keys.sort();
        keys
    }
}
