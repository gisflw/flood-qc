use std::{env, fs, path::PathBuf};
fn main() {
    if env::args().nth(1).as_deref() == Some("lock") {
        use std::io::{Read, Write};
        let file = fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(env::args().nth(2).unwrap()).unwrap();
        file.try_lock().unwrap();
        std::io::stdout().write_all(b"locked\n").unwrap();
        let mut byte = [0];
        let _ = std::io::stdin().read(&mut byte);
        return;
    }
    let input = PathBuf::from(env::var_os("MGB_INPUT_DIR").unwrap());
    let output = PathBuf::from(env::var_os("MGB_OUTPUT_DIR").unwrap());
    let scenario = input.parent().unwrap().file_name().unwrap().to_string_lossy();
    let control = fs::read_to_string(input.join("control.txt")).unwrap_or_default();
    if control.lines().any(|line| line == format!("fail:{scenario}")) { std::process::exit(3); }
    if control.lines().any(|line| line == format!("large:{scenario}")) {
        use std::io::Write;
        std::io::stdout().write_all(&vec![b'x'; 2_000_000]).unwrap();
        std::io::stderr().write_all(&vec![b'y'; 2_000_000]).unwrap();
    }
    let meta = fs::read_to_string(input.join("Previsao.meta")).unwrap();
    let lines: Vec<_> = meta.lines().collect();
    let header = lines.iter().position(|line| line.contains("HORIZ(NT)")).unwrap();
    let row: Vec<_> = lines[header + 1].split_whitespace().collect();
    let horizon: usize = row[4].parse().unwrap();
    let par = fs::read_to_string(input.join("PARHIG.hig")).unwrap();
    let lines: Vec<_> = par.lines().collect();
    let header = lines.iter().position(|line| line.contains("NT") && line.contains("DT")).unwrap();
    let total: usize = lines[header + 1].split_whitespace().next().unwrap().parse().unwrap();
    let history = total - horizon;
    let binary = |count: usize, value: f32| -> Vec<u8> { (0..count).flat_map(|_| value.to_le_bytes()).collect() };
    fs::write(output.join("QTUDO_Inercial_Atual.MGB"), binary(history, 2.0)).unwrap();
    fs::write(output.join("QTUDO_Inercial_Prev.MGB"), binary(horizon, 3.0)).unwrap();
    fs::write(output.join("YTUDO.MGB"), binary(history, 1.0041)).unwrap();
    fs::write(output.join("YTUDO_prev.MGB"), binary(horizon, 1.0041)).unwrap();
    if control.lines().any(|line| line == format!("malformed:{scenario}")) {
        fs::write(output.join("YTUDO_prev.MGB"), [1]).unwrap();
    }
    if scenario != ".history" {
        fs::write(input.parent().unwrap().join("finished"), b"done").unwrap();
    }
    if control.lines().any(|line| line == format!("manifest:{scenario}")) {
        fs::write(input.ancestors().nth(3).unwrap().join("manifest.json.tmp"), b"injected failure").unwrap();
    }
}
