use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AnaPartialCoverage {
    stations: BTreeMap<String, PartialDay>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PartialDay {
    day: NaiveDate,
    through_utc: i64,
}
impl AnaPartialCoverage {
    fn key(station: &str, variables: &[String]) -> Result<String, String> {
        let mut variables = variables.to_vec();
        variables.sort();
        variables.dedup();
        serde_json::to_string(&(station, variables)).map_err(|e| e.to_string())
    }
    pub(crate) fn covers(
        &self,
        station: &str,
        variables: &[String],
        day: NaiveDate,
        through_utc: i64,
    ) -> Result<bool, String> {
        Ok(self
            .stations
            .get(&Self::key(station, variables)?)
            .is_some_and(|entry| entry.day == day && entry.through_utc >= through_utc))
    }
    pub(crate) fn record(
        &mut self,
        station: &str,
        variables: &[String],
        day: NaiveDate,
        through_utc: i64,
    ) -> Result<(), String> {
        let key = Self::key(station, variables)?;
        self.stations
            .entry(key)
            .and_modify(|entry| {
                if entry.day != day {
                    *entry = PartialDay { day, through_utc };
                } else {
                    entry.through_utc = entry.through_utc.max(through_utc);
                }
            })
            .or_insert(PartialDay { day, through_utc });
        Ok(())
    }
}
