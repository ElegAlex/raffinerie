//! Filter evaluation engine — Task 9 implements this.

use crate::filter::set::FilterSet;
use crate::parser::ParsedDump;
use crate::schema::{Creance, CreanceRegroupee};
use chrono::NaiveDate;

pub struct FilteredRow<'a> {
    pub creance: &'a Creance,
    pub regroupee: Option<&'a CreanceRegroupee>,
    pub pivot_date: Option<NaiveDate>,
}

pub fn evaluate<'a>(_dump: &'a ParsedDump, _fs: &FilterSet) -> Vec<FilteredRow<'a>> {
    Vec::new()
}
