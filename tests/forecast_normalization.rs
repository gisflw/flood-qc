use chrono::TimeZone;
use mgb_ops::{
    adapters::{NormalizationRequest, normalize_saved_forecast},
    store::ForecastProvider,
};
use std::path::Path;
fn request(provider: ForecastProvider) -> NormalizationRequest {
    NormalizationRequest {
        provider,
        model: match provider {
            ForecastProvider::Ecmwf => "ifs",
            ForecastProvider::Noaa => "gfs",
        }
        .into(),
        product: "fc".into(),
        cycle_utc: chrono::Utc.with_ymd_and_hms(2026, 7, 20, 0, 0, 0).unwrap(),
        operation_timezone: chrono_tz::UTC,
        crop_bbox: [-0.1, -0.1, 0.35, 0.35],
    }
}
#[test]
fn saved_simple_and_ccsds_grib_normalize_two_native_intervals() {
    for (name, provider) in [
        ("noaa", ForecastProvider::Noaa),
        ("ecmwf", ForecastProvider::Ecmwf),
    ] {
        let file = format!("tests/fixtures/forecast/{name}.grib2");
        let result = normalize_saved_forecast(Path::new(&file), &request(provider)).unwrap();
        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.native.timeline.bounds.len(), 2);
        assert_eq!(result.native.grid.latitudes.len(), 2);
        assert_eq!(result.native.grid.longitudes.len(), 2);
        assert_eq!(result.native.values.len(), 8);
        assert!(result.native.valid.iter().all(|v| *v));
        assert!(result.native.values.iter().all(|v| *v >= 0.));
    }
}
#[test]
fn duplicates_rolling_fields_and_changed_grids_are_handled_explicitly() {
    let req = request(ForecastProvider::Noaa);
    let original =
        normalize_saved_forecast(Path::new("tests/fixtures/forecast/noaa.grib2"), &req).unwrap();
    let extra = normalize_saved_forecast(
        Path::new("tests/fixtures/forecast/noaa-duplicate-rolling.grib2"),
        &req,
    )
    .unwrap();
    assert_eq!(original.native.values, extra.native.values);
    assert_eq!(extra.messages.iter().filter(|m| !m.selected).count(), 1);
    let conflict = normalize_saved_forecast(
        Path::new("tests/fixtures/forecast/noaa-conflicting.grib2"),
        &req,
    );
    let duplicate = matches!(
        conflict,
        Err(mgb_ops::adapters::NormalizationError::ConflictingDuplicate(
            _
        ))
    );
    assert!(duplicate);
    let changed = normalize_saved_forecast(
        Path::new("tests/fixtures/forecast/noaa-changed-grid.grib2"),
        &req,
    );
    let mismatch = matches!(
        changed,
        Err(mgb_ops::adapters::NormalizationError::GridMismatch(_))
    );
    assert!(mismatch);
}

#[test]
fn rejects_unqualified_discipline_pdt_surface_local_table_and_cycle() {
    fn mutate(section: u8, offset: usize, value: u8, input: &[u8]) -> Vec<u8> {
        let mut bytes = input.to_vec();
        let mut message = 0;
        while message < bytes.len() {
            let size =
                u64::from_be_bytes(bytes[message + 8..message + 16].try_into().unwrap()) as usize;
            if section == 0 {
                bytes[message + offset] = value;
            } else {
                let mut pos = message + 16;
                while pos < message + size - 4 {
                    let n = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
                    if bytes[pos + 4] == section {
                        bytes[pos + offset] = value;
                        break;
                    }
                    pos += n;
                }
            }
            message += size;
        }
        bytes
    }
    let dir = tempfile::tempdir().unwrap();
    let noaa = std::fs::read("tests/fixtures/forecast/noaa.grib2").unwrap();
    let ecmwf = std::fs::read("tests/fixtures/forecast/ecmwf.grib2").unwrap();
    for (index, bytes, provider) in [
        (0, mutate(0, 6, 10, &noaa), ForecastProvider::Noaa),
        (1, mutate(4, 41, 2, &noaa), ForecastProvider::Noaa),
        (2, mutate(4, 28, 1, &noaa), ForecastProvider::Noaa),
        (3, mutate(1, 10, 10, &ecmwf), ForecastProvider::Ecmwf),
        (4, mutate(4, 8, 0, &noaa), ForecastProvider::Noaa),
    ] {
        let path = dir.path().join(format!("bad-{index}.grib2"));
        std::fs::write(&path, bytes).unwrap();
        assert!(
            normalize_saved_forecast(&path, &request(provider)).is_err(),
            "mutation {index}"
        );
    }
    let mut wrong = request(ForecastProvider::Noaa);
    wrong.cycle_utc += chrono::Duration::hours(1);
    assert!(
        normalize_saved_forecast(Path::new("tests/fixtures/forecast/noaa.grib2"), &wrong).is_err()
    );
}
#[test]
fn lead_zero_missing_cells_and_negative_increments_remain_explicit() {
    let result = normalize_saved_forecast(
        Path::new("tests/fixtures/forecast/noaa-missing-clipped.grib2"),
        &request(ForecastProvider::Noaa),
    )
    .unwrap();
    assert_eq!(result.native.timeline.bounds.len(), 2);
    assert_eq!(result.clipped_negative_count, 1);
    assert_eq!(result.clipped_negative_total_mm, 3.);
    assert_eq!(
        result.native.provenance.processing["clipped_negative_count"],
        1
    );
    assert_eq!(
        result.native.provenance.processing["clipped_negative_total_mm"],
        3.
    );
    assert_eq!(&result.native.values[..3], [3., 6., 0.]);
    assert!(!result.native.valid[3]);
    assert!(result.native.values[3].is_nan());
    assert_eq!(&result.native.values[4..7], [6., 0., 0.]);
    assert!(!result.native.valid[7]);
    assert!(result.native.values[7].is_nan());
}
