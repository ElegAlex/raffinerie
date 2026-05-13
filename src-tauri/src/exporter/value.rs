use chrono::NaiveDate;
use rust_xlsxwriter::{Format, Worksheet, XlsxError};

#[derive(Debug, Clone)]
pub enum Value {
    Empty,
    Text(String),
    Int(i64),
    Money(f64),
    Date(NaiveDate),
    Bool(bool),
}

pub fn write(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    v: &Value,
    money_fmt: &Format,
    date_fmt: &Format,
) -> Result<(), XlsxError> {
    match v {
        Value::Empty => Ok(()),
        Value::Text(s) => {
            ws.write_string(row, col, s)?;
            Ok(())
        }
        Value::Int(i) => {
            ws.write_number(row, col, *i as f64)?;
            Ok(())
        }
        Value::Money(f) => {
            ws.write_number_with_format(row, col, *f, money_fmt)?;
            Ok(())
        }
        Value::Date(d) => {
            ws.write_with_format(row, col, d, date_fmt)?;
            Ok(())
        }
        Value::Bool(b) => {
            ws.write_string(row, col, if *b { "Oui" } else { "Non" })?;
            Ok(())
        }
    }
}

pub fn money_format() -> Format {
    Format::new().set_num_format("# ##0,00 €")
}

pub fn date_format() -> Format {
    Format::new().set_num_format("dd/mm/yyyy")
}

pub fn header_format() -> Format {
    Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0xE0E0E0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_xlsxwriter::Workbook;

    #[test]
    fn write_each_value_kind() {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        let money = money_format();
        let date = date_format();
        write(ws, 0, 0, &Value::Empty, &money, &date).unwrap();
        write(ws, 0, 1, &Value::Text("hello".into()), &money, &date).unwrap();
        write(ws, 0, 2, &Value::Int(42), &money, &date).unwrap();
        write(ws, 0, 3, &Value::Money(1234.56), &money, &date).unwrap();
        write(
            ws,
            0,
            4,
            &Value::Date(NaiveDate::from_ymd_opt(2026, 5, 13).unwrap()),
            &money,
            &date,
        )
        .unwrap();
        write(ws, 0, 5, &Value::Bool(true), &money, &date).unwrap();
    }
}
