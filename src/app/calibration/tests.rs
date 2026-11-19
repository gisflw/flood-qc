use super::*;
use std::path::PathBuf;

fn test_dir() -> tempfile::TempDir {
    fs::create_dir_all("runs").unwrap();
    tempfile::tempdir_in("runs").unwrap()
}

fn synthetic_geometry() -> Vec<u8> {
    const WIDTHS: [usize; 19] = [
        8, 8, 15, 15, 8, 15, 15, 15, 15, 15, 15, 8, 8, 5, 10, 10, 10, 10, 10,
    ];
    let row = |values: [&str; 19]| {
        let line: String = values
            .iter()
            .zip(WIDTHS)
            .map(|(value, width)| format!("{value:>width$}"))
            .collect();
        line
    };
    let header = row([
        "CatID",
        "Mini",
        "Xcen",
        "Ycen",
        "Sub",
        "Area_(km2)",
        "AreaM_(km2)",
        "Ltr_(km)",
        "Str_(m/km)",
        "Lrl_(km)",
        "Srl_(m/km)",
        "MiniJus",
        "Ordem",
        "Hdr",
        "Width",
        "Depth",
        "NMAN",
        "BLC_01",
        "BLC_02",
    ]);
    let a = row([
        "3",
        "1",
        "-50.0",
        "-25.0",
        "1",
        "4.0",
        "4.0",
        "1.5",
        "0.01",
        "0.0",
        "0.01",
        "2",
        "1",
        "1",
        "8.10",
        "3.60",
        "0.0300",
        "100.00000",
        "0.00000",
    ]);
    let b = row([
        "5", "2", "-51.0", "-26.0", "3", "5.0", "9.0", "1.0", "0.02", "2.50", "0.01", "-1", "2",
        "0", "10.50", "4.00", "0.0320", "23.00000", "77.00000",
    ]);
    format!("{header}\r\n{a}\r\n{b}\r\n").into_bytes()
}

#[test]
fn geometry_edits_are_atomic_and_save_preserves_fixed_fields() {
    let dir = test_dir();
    let src = dir.path().join("MINI.gtp");
    let output = dir.path().join("edited.gtp");
    let table = dir.path().join("channels.csv");
    let original = synthetic_geometry();
    fs::write(&src, &original).unwrap();
    let mut geom = CalibrationGeometry::read(&src).unwrap();
    assert_eq!(geom.block_count(), 2);
    assert_eq!(
        geom.edit(GeometryEdit::Manning {
            subbasin: Some(3),
            value: 0.04
        }),
        Ok(1)
    );
    assert_eq!(geom.rows()[0].manning, 0.03);
    assert_eq!(geom.rows()[1].manning, 0.04);

    assert!(
        geom.edit(GeometryEdit::Cell {
            mini: 2,
            field: GeometryField::Downstream,
            value: 1.0
        })
        .is_err()
    );
    assert!(
        geom.edit(GeometryEdit::Cell {
            mini: 1,
            field: GeometryField::Hdr,
            value: 2.0
        })
        .is_err()
    );
    assert!(
        geom.edit(GeometryEdit::Cell {
            mini: 1,
            field: GeometryField::Block(0),
            value: 0.0
        })
        .is_err()
    );
    assert!(
        geom.edit(GeometryEdit::Cell {
            mini: 1,
            field: GeometryField::Area,
            value: f64::NAN
        })
        .is_err()
    );
    assert!(
        geom.edit(GeometryEdit::Manning {
            subbasin: Some(2),
            value: 0.03
        })
        .is_err()
    );
    assert!(
        geom.edit(GeometryEdit::Equation {
            dimension: ChannelDimension::Width,
            a: 1e100,
            b: 0.5
        })
        .is_err()
    );

    fs::write(&table, "1;50;30\n9;100;40\n").unwrap();
    assert!(geom.apply_channel_table(&table).is_err());
    assert_eq!(geom.rows()[0].width, 8.1);
    fs::write(&table, "1;50;30\n1;100;40\n").unwrap();
    assert!(geom.apply_channel_table(&table).is_err());
    assert_eq!(geom.rows()[0].width, 8.1);
    fs::write(&table, "Mini;Width;Depth\n1;50;30\n").unwrap();
    assert_eq!(geom.apply_channel_table(&table), Ok(1));
    assert_eq!(
        geom.edit(GeometryEdit::Equation {
            dimension: ChannelDimension::Depth,
            a: 3.0,
            b: 0.5
        }),
        Ok(2)
    );
    assert_eq!(geom.rows()[0].depth, 6.0);
    assert_eq!(geom.rows()[1].depth, 9.0);
    assert_eq!(geom.rows()[1].width, 10.5);
    assert_eq!(
        geom.edit(GeometryEdit::Manning {
            subbasin: None,
            value: 0.04
        }),
        Ok(1)
    );

    geom.save(&output).unwrap();
    assert!(geom.save(&src).is_err());
    assert!(geom.save(&output).is_err());
    assert_eq!(fs::read(&src).unwrap(), original);
    let edited = fs::read(&output).unwrap();
    let len = 217;
    assert_eq!(&edited[..len + 165], &original[..len + 165]);
    assert_eq!(
        &edited[len + 195..len * 2 + 175],
        &original[len + 195..len * 2 + 175]
    );
    assert_eq!(&edited[len * 2 + 195..], &original[len * 2 + 195..]);
    assert_eq!(
        CalibrationGeometry::read(&output).unwrap().rows(),
        geom.rows()
    );

    let block_output = dir.path().join("edited-blocks.gtp");
    geom.edit(GeometryEdit::Cell {
        mini: 1,
        field: GeometryField::Block(1),
        value: 99.0,
    })
    .unwrap();
    assert!(geom.save(&block_output).is_err());
    geom.edit(GeometryEdit::Cell {
        mini: 1,
        field: GeometryField::Block(2),
        value: 1.0,
    })
    .unwrap();
    geom.save(&block_output).unwrap();

    for (range, value) in [
        (len..len + 8, b"      xx"),
        (2 * len + 8..2 * len + 16, b"       1"),
        (len + 144..len + 152, b"     2.0"),
    ] {
        let mut bad = original.clone();
        bad[range].copy_from_slice(value);
        fs::write(&output, bad).unwrap();
        assert!(CalibrationGeometry::read(&output).is_err());
    }
}

#[test]
fn soil_validates_dense_blocks_and_supports_copy_and_staging() {
    let dir = test_dir();
    let src = dir.path().join("Cell.hig");
    let hrc = dir.path().join("HRC.hrc");
    let original = synthetic_geometry();
    fs::write(&src, &original).unwrap();
    fs::write(dir.path().join("MINI.gtp"), &original).unwrap();
    fs::write(dir.path().join("MINI.mgb"), &original).unwrap();
    fs::write(&hrc, b"Land // \xE1rea verde\r\nWater // Agua\r\n").unwrap();
    let mut geom = CalibrationGeometry::read(&src).unwrap();
    let mut soil = SoilCalibration::create(&geom, &hrc).unwrap();
    assert_eq!(soil.classes()[0].description, "área verde");
    assert_eq!(
        soil.watersheds().iter().map(|w| w.id).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    let baseline_soil = dir.path().join("paruso.hig");
    soil.save(&baseline_soil).unwrap();
    let original_soil = fs::read(&baseline_soil).unwrap();
    let values = [50.0, 0.2, 1.0, 12.0, 0.67, 0.0, 0.30];
    let coefficients = [24.0, 45.0, 5000.0, 0.003];
    assert!(
        soil.edit(SoilEdit::Class {
            watershed: 2,
            class: 1,
            values: [f64::INFINITY; 7]
        })
        .is_err()
    );
    assert!(
        soil.edit(SoilEdit::Coefficients {
            watershed: 2,
            values: [2.0, 3.0, -1.0, 0.0]
        })
        .is_err()
    );
    assert!(
        soil.edit(SoilEdit::Coefficients {
            watershed: 0,
            values: [0.0; 4]
        })
        .is_err()
    );
    assert_eq!(soil.watersheds()[1].classes, [[0.0; 7], [0.0; 7]]);
    assert_eq!(soil.watersheds()[1].coefficients, [0.0; 4]);
    assert_eq!(
        soil.edit(SoilEdit::Class {
            watershed: 2,
            class: 2,
            values
        }),
        Ok(1)
    );
    assert_eq!(
        soil.edit(SoilEdit::Coefficients {
            watershed: 2,
            values: coefficients
        }),
        Ok(1)
    );
    assert_eq!(soil.edit(SoilEdit::CopyWatershed { from: 2, to: 3 }), Ok(1));
    assert_eq!(
        soil.edit(SoilEdit::CopyToAllClasses {
            watershed: 3,
            from: 2
        }),
        Ok(1)
    );
    assert_eq!(soil.watersheds()[2].classes, [values, values]);
    assert_eq!(soil.watersheds()[2].coefficients, coefficients);
    assert_eq!(soil.edit(SoilEdit::CopyToAllWatersheds { from: 3 }), Ok(2));

    let selected_soil = dir.path().join("selected.CAL");
    let selected_geom = dir.path().join("selected.gtp");
    soil.save(&selected_soil).unwrap();
    assert_eq!(
        SoilCalibration::read(&selected_soil, &geom, &hrc)
            .unwrap()
            .watersheds(),
        soil.watersheds()
    );
    let matches_source = soil
        .watersheds()
        .iter()
        .all(|w| w.classes == [values, values] && w.coefficients == coefficients);
    assert!(matches_source);
    geom.edit(GeometryEdit::Cell {
        mini: 1,
        field: GeometryField::Hdr,
        value: 0.0,
    })
    .unwrap();
    geom.save(&selected_geom).unwrap();
    validate_selected_calibration(dir.path(), Some(&selected_geom), Some(&selected_soil)).unwrap();

    let bad = dir.path().join("invalid.CAL");
    let bytes = fs::read(&selected_soil).unwrap();
    let qb_label = b"QB_M3/SKM2";
    let qb = bytes
        .windows(qb_label.len())
        .position(|window| window == qb_label)
        .unwrap()
        + qb_label.len();
    assert_eq!(&bytes[qb..qb + 8], b"  0.0030");
    fs::write(
        &bad,
        String::from_utf8(bytes.clone())
            .unwrap()
            .replacen("Land", "Other", 1),
    )
    .unwrap();
    assert!(SoilCalibration::read(&bad, &geom, &hrc).is_err());
    assert!(
        crate::model::install_calibration(dir.path(), Some(&selected_geom), Some(&bad)).is_err()
    );
    assert_eq!(fs::read(&src).unwrap(), original);
    assert_eq!(fs::read(&baseline_soil).unwrap(), original_soil);

    let mut incompatible_geometry = fs::read(&selected_geom).unwrap();
    incompatible_geometry[217 + 46..217 + 54].copy_from_slice(b"       2");
    let bad_geometry = dir.path().join("invalid.gtp");
    fs::write(&bad_geometry, incompatible_geometry).unwrap();
    assert!(validate_selected_calibration(dir.path(), Some(&bad_geometry), None).is_err());

    crate::model::install_calibration(dir.path(), Some(&selected_geom), Some(&selected_soil))
        .unwrap();
    let geometry = fs::read(&selected_geom).unwrap();
    for alias in ["Cell.hig", "MINI.gtp", "MINI.mgb"] {
        assert_eq!(fs::read(dir.path().join(alias)).unwrap(), geometry);
    }
    assert_eq!(fs::read(&baseline_soil).unwrap(), bytes);
    crate::model::install_calibration(&dir.path().join("absent"), None, None).unwrap();
}

#[test]
#[ignore = "requires MGB_CALIBRATION_RS_INPUT pointing to the regional runtime Input directory"]
fn rs_calibration_bytes_are_preserved() {
    let input = PathBuf::from(std::env::var_os("MGB_CALIBRATION_RS_INPUT").unwrap());
    let dir = test_dir();
    let cell = input.join("Cell.hig");
    let paruso = input.join("paruso.hig");
    let hrc = input.join("HRC_description.hrc");
    let cell_bytes = fs::read(&cell).unwrap();
    let paruso_bytes = fs::read(&paruso).unwrap();
    let mut geometry = CalibrationGeometry::read(&cell).unwrap();
    assert_eq!(geometry.block_count(), 9);
    assert_eq!(geometry.rows().len(), 8649);
    assert_eq!(geometry.rows()[0].hdr, 1);
    let missing_subbasin = geometry.rows().iter().all(|row| row.subbasin != 7);
    assert!(missing_subbasin);
    let mut soil = SoilCalibration::read(&paruso, &geometry, &hrc).unwrap();
    assert_eq!(soil.watersheds().len(), 135);
    assert_eq!(soil.watersheds()[6].id, 7);
    assert_eq!(soil.classes()[8].description, "Água");

    let original_geo = dir.path().join("original.gtp");
    let original_soil = dir.path().join("original.CAL");
    geometry.save(&original_geo).unwrap();
    soil.save(&original_soil).unwrap();
    assert_eq!(fs::read(&original_geo).unwrap(), cell_bytes);
    assert_eq!(fs::read(&original_soil).unwrap(), paruso_bytes);

    geometry
        .edit(GeometryEdit::Cell {
            mini: 1,
            field: GeometryField::Width,
            value: 8.2,
        })
        .unwrap();
    soil.edit(SoilEdit::Class {
        watershed: 1,
        class: 1,
        values: [51.0, 0.05, 0.50, 11.00, 0.67, 0.0, 0.20],
    })
    .unwrap();
    let edited_geo = dir.path().join("edited.gtp");
    let edited_soil = dir.path().join("edited.CAL");
    geometry.save(&edited_geo).unwrap();
    soil.save(&edited_soil).unwrap();
    let mut expected_geo = cell_bytes.clone();
    expected_geo[287 + 165..287 + 175].copy_from_slice(b"      8.20");
    assert_eq!(fs::read(&edited_geo).unwrap(), expected_geo);
    let mut expected_soil = paruso_bytes.clone();
    let first_wm = b"Watershed 1\r\n".len() + 68 + 9;
    expected_soil[first_wm..first_wm + 9].copy_from_slice(b"     51.0");
    assert_eq!(fs::read(&edited_soil).unwrap(), expected_soil);
    validate_selected_calibration(&input, Some(&edited_geo), Some(&edited_soil)).unwrap();
    for alias in ["Cell.hig", "MINI.gtp", "MINI.mgb"] {
        assert_eq!(fs::read(input.join(alias)).unwrap(), cell_bytes);
    }
    assert_eq!(fs::read(&paruso).unwrap(), paruso_bytes);
}
