use mgb_ops::app::{
    AcquisitionReport, DownloadEvent, ExecutionReport, OperationControl, OperationError,
    OperationEvent, OperationLogStream, OperationStage, OperationalRunReport, PreparationReport,
};
use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    io::{self, IsTerminal, Write},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const MAX_PENDING_BYTES: usize = 1024 * 1024;
const MAX_LINE_BYTES: usize = 16 * 1024;

type LogKey = (Option<String>, &'static str);

fn stream_name(stream: OperationLogStream) -> &'static str {
    match stream {
        OperationLogStream::Stdout => "stdout",
        OperationLogStream::Stderr => "stderr",
    }
}

enum ConsoleEvent {
    Milestone(OperationEvent),
    Output(LogKey, Vec<u8>),
    Skipped(LogKey, usize),
}

#[derive(Default)]
struct Pending {
    events: VecDeque<ConsoleEvent>,
    log_bytes: usize,
    skipped: BTreeMap<LogKey, usize>,
}

impl Pending {
    fn report_skipped(&mut self, key: &LogKey) {
        if let Some(bytes) = self.skipped.remove(key) {
            self.events
                .push_back(ConsoleEvent::Skipped(key.clone(), bytes));
        }
    }

    fn push(&mut self, event: OperationEvent) {
        match event {
            OperationEvent::EngineOutput {
                scenario,
                stream,
                bytes,
            } => {
                let key = (scenario, stream_name(stream));
                if self.log_bytes + bytes.len() > MAX_PENDING_BYTES {
                    let skipped = self.skipped.entry(key).or_default();
                    *skipped = skipped.saturating_add(bytes.len());
                } else {
                    self.report_skipped(&key);
                    self.log_bytes += bytes.len();
                    self.events.push_back(ConsoleEvent::Output(key, bytes));
                }
            }
            OperationEvent::EngineFinished { ref scenario } => {
                for stream in ["stdout", "stderr"] {
                    self.report_skipped(&(scenario.clone(), stream));
                }
                self.events.push_back(ConsoleEvent::Milestone(event));
            }
            _ => self.events.push_back(ConsoleEvent::Milestone(event)),
        }
    }

    fn pop(&mut self) -> Option<ConsoleEvent> {
        let event = self.events.pop_front()?;
        if let ConsoleEvent::Output(_, bytes) = &event {
            self.log_bytes -= bytes.len();
        }
        Some(event)
    }
}

struct Console<'a> {
    out: &'a mut dyn Write,
    error: Option<io::Error>,
    partial: BTreeMap<LogKey, Vec<u8>>,
    selection_shown: bool,
    terminal: bool,
    activity: Option<Activity>,
}

struct Activity {
    detail: String,
    status: &'static str,
    frame: usize,
    drawn: Instant,
}

impl Console<'_> {
    fn write(&mut self, text: fmt::Arguments<'_>) {
        if self.error.is_none() {
            self.error = (|| {
                self.out.write_fmt(text)?;
                self.out.flush()
            })()
            .err();
        }
    }

    fn line(&mut self, text: fmt::Arguments<'_>) {
        self.clear_activity();
        self.write(format_args!("{text}\n"));
    }

    fn clear_activity(&mut self) {
        if self.activity.take().is_some() {
            self.write(format_args!("\r\x1b[2K\x1b[1A\r\x1b[2K"));
        }
    }

    fn show_activity(&mut self, detail: String, status: &'static str) {
        if self.terminal && self.error.is_none() {
            let frame = self.activity.as_ref().map_or(0, |activity| activity.frame);
            self.clear_activity();
            let spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            self.write(format_args!("{detail}\n{} {status}", spinner[frame]));
            self.activity = Some(Activity {
                detail,
                status,
                frame: (frame + 1) % spinner.len(),
                drawn: Instant::now(),
            });
        }
    }

    fn tick(&mut self) {
        if let Some(activity) = &self.activity
            && activity.drawn.elapsed() >= Duration::from_millis(100)
        {
            self.show_activity(activity.detail.clone(), activity.status);
        }
    }

    fn download(&mut self, event: DownloadEvent) {
        let now = chrono::Local::now();
        let time = now.format("%Y-%m-%d %H:%M:%S");
        match event {
            DownloadEvent::Started(reference) => {
                self.line(format_args!("Acquiring inputs for {reference}"));
            }
            DownloadEvent::AnaWindow {
                station,
                start,
                end,
                rows,
                loading,
            } => {
                let status = if loading { "loading" } else { "requesting" };
                self.show_activity(
                    format!("{time} | ANA {station} | {start}, {end} | rows {rows}"),
                    status,
                );
            }
            DownloadEvent::AnaFinished {
                station,
                dates,
                rows,
                changed_samples,
                error,
                cancelled,
            } => {
                let status = match (cancelled, error.is_some(), rows) {
                    (true, _, _) => "interrupted",
                    (_, true, _) => "failed",
                    (_, _, 0) => "no data",
                    _ => "complete",
                };
                let window =
                    dates.map_or_else(|| "-".to_owned(), |(start, end)| format!("{start}, {end}"));
                let reason = error.map_or_else(String::new, |error| format!(" | {error}"));
                self.line(format_args!("{time} | ANA {station} | {status} | update {window} | rows {rows} | changed {changed_samples}{reason}"));
            }
            DownloadEvent::AnaFailed(error) => self.line(format_args!("ANA failed: {error}")),
            DownloadEvent::ForecastCycle { provider, cycle } => {
                let provider = format!("{provider:?}").to_ascii_uppercase();
                self.line(format_args!("{time} | {provider} | checking {cycle}"));
            }
            DownloadEvent::ForecastLead {
                provider,
                cycle,
                lead,
                index,
                total,
            } => {
                let provider = format!("{provider:?}").to_ascii_uppercase();
                self.show_activity(
                    format!("{provider} | {cycle} | +{lead} h ({index}/{total})"),
                    "downloading",
                );
            }
            DownloadEvent::ForecastFinished(outcome) => {
                let provider = format!("{:?}", outcome.provider).to_ascii_uppercase();
                if let Some(error) = outcome.error {
                    self.line(format_args!("{time} | {provider} | failed: {error}"));
                } else if let Some(asset) = outcome.asset_id {
                    let action = if outcome.reused {
                        "reused"
                    } else {
                        "registered"
                    };
                    self.line(format_args!("{time} | {provider} | {action} {asset}"));
                    for path in [outcome.retained_source, outcome.native_artifact]
                        .into_iter()
                        .flatten()
                    {
                        self.line(format_args!("  {}", path.display()));
                    }
                }
            }
        }
    }

    fn log_line(&mut self, key: &LogKey, bytes: &[u8]) {
        let stream = key.1;
        let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
        let mut text = String::new();
        for character in String::from_utf8_lossy(bytes).chars() {
            if character.is_control() && character != '\t' {
                text.extend(character.escape_debug());
            } else {
                text.push(character);
            }
        }
        match &key.0 {
            Some(id) => self.line(format_args!("[MGB scenario {id} {stream}] {text}")),
            None => self.line(format_args!("[MGB history {stream}] {text}")),
        }
    }

    fn log_bytes(&mut self, key: LogKey, bytes: &[u8]) {
        let mut partial = self.partial.remove(&key).unwrap_or_default();
        for line in bytes.split_inclusive(|byte| *byte == b'\n') {
            let mut content = line.strip_suffix(b"\n").unwrap_or(line);
            while content.len() > MAX_LINE_BYTES - partial.len() {
                let (prefix, rest) = content.split_at(MAX_LINE_BYTES - partial.len());
                partial.extend_from_slice(prefix);
                self.log_line(&key, &partial);
                partial.clear();
                content = rest;
            }
            partial.extend_from_slice(content);
            if line.ends_with(b"\n") {
                self.log_line(&key, &partial);
                partial.clear();
            }
        }
        if !partial.is_empty() {
            self.partial.insert(key, partial);
        }
    }

    fn finish_stream(&mut self, key: &LogKey) {
        if let Some(partial) = self.partial.remove(key) {
            self.log_line(key, &partial);
        }
    }

    fn event(&mut self, event: ConsoleEvent) {
        if self.error.is_some() {
            return;
        }
        match event {
            ConsoleEvent::Output(key, bytes) => self.log_bytes(key, &bytes),
            ConsoleEvent::Skipped(key, count) => {
                self.finish_stream(&key);
                self.log_line(
                    &key,
                    format!("Skipped {count} bytes of console output").as_bytes(),
                );
            }
            ConsoleEvent::Milestone(event) => match event {
                OperationEvent::Stage(stage) => {
                    let message = match stage {
                        OperationStage::Acquisition => "Acquiring inputs",
                        OperationStage::Preparation => "Preparing operation",
                        OperationStage::History => "Running shared MGB history",
                        OperationStage::Scenarios => "Running forecast scenarios",
                        OperationStage::Finalization => "Finalizing run",
                    };
                    self.line(format_args!("{message}"));
                }
                OperationEvent::Download(event) => self.download(event),
                OperationEvent::Selection {
                    reference_time,
                    time_zone,
                    scenario_ids,
                } => {
                    if !self.selection_shown {
                        self.line(format_args!(
                            "Reference time: {reference_time} ({time_zone})"
                        ));
                        self.line(format_args!("Scenarios: {}", scenario_ids.join(", ")));
                        self.selection_shown = true;
                    }
                }
                OperationEvent::Scenario(scenario) => {
                    if scenario.state != "pending" {
                        self.line(format_args!("Scenario {}: {}", scenario.id, scenario.state));
                    }
                }
                OperationEvent::EngineFinished { scenario } => {
                    self.finish_stream(&(scenario.clone(), "stdout"));
                    self.finish_stream(&(scenario, "stderr"));
                }
                OperationEvent::EngineOutput { .. } => unreachable!("queued as console output"),
            },
        }
    }
}

pub(super) trait Summary {
    fn print(&self, line: impl FnMut(fmt::Arguments<'_>));
}

impl Summary for mgb_ops::store::InputSetManifest {
    fn print(&self, mut line: impl FnMut(fmt::Arguments<'_>)) {
        line(format_args!("Captured inputs: {}", self.id));
    }
}
impl Summary for PreparationReport {
    fn print(&self, mut line: impl FnMut(fmt::Arguments<'_>)) {
        line(format_args!("Prepared: {}", self.prepared.display()));
    }
}

impl Summary for ExecutionReport {
    fn print(&self, mut line: impl FnMut(fmt::Arguments<'_>)) {
        line(format_args!(
            "Run: {} ({:?})",
            self.run_root.display(),
            self.status
        ));
        line(format_args!(
            "Run artifact: {}",
            self.run_artifact.display()
        ));
    }
}

pub(super) fn run<T: Summary + Send>(
    stdout: &mut io::StdoutLock<'_>,
    json_stdout: bool,
    action: impl FnOnce(&OperationControl) -> Result<T, OperationError> + Send,
) -> io::Result<Result<T, OperationError>> {
    Progress::new()?.run(stdout, json_stdout, action)
}

pub(super) fn acquire(
    stdout: &mut io::StdoutLock<'_>,
    json_stdout: bool,
    action: impl FnOnce(&OperationControl) -> Result<AcquisitionReport, String> + Send,
) -> io::Result<Result<AcquisitionReport, String>> {
    Progress::new()?.observe(stdout, json_stdout, action, |console, result| {
        if let Ok(report) = result {
            console.acquisition_summary(report);
        }
    })
}

pub(super) fn run_all(
    stdout: &mut io::StdoutLock<'_>,
    json_stdout: bool,
    action: impl FnOnce(&OperationControl) -> Result<OperationalRunReport, String> + Send,
) -> io::Result<Result<OperationalRunReport, String>> {
    Progress::new()?.observe(stdout, json_stdout, action, |console, result| {
        if let Ok(report) = result {
            if let Some(acquisition) = &report.acquisition
                && acquisition.cancelled
            {
                console.line(format_args!("Acquisition cancelled"));
            }
            if let Some(execution) = &report.execution {
                execution.print(|line| console.line(line));
            }
            if let Some(error) = &report.error {
                if matches!(error, OperationError::BatchFailed { .. }) {
                    console.operation_failure(error);
                } else {
                    console.line(format_args!("{error}"));
                }
            }
        }
    })
}

impl Console<'_> {
    fn acquisition_summary(&mut self, report: &AcquisitionReport) {
        let summary = if report.cancelled {
            "Acquisition cancelled"
        } else if report.is_complete() {
            "Acquisition complete"
        } else {
            "Acquisition finished with errors"
        };
        self.line(format_args!("{summary}"));
    }

    fn operation_failure(&mut self, error: &OperationError) {
        if let OperationError::BatchFailed { report } = error {
            if let Some(error) = &report.error {
                self.line(format_args!("Run failed: {error}"));
            }
            self.line(format_args!(
                "Run artifact: {}",
                report.run_artifact.display()
            ));
        }
    }
}

struct Progress {
    pending: Arc<Mutex<Pending>>,
    control: OperationControl,
}

impl Progress {
    fn new() -> io::Result<Self> {
        let pending = Arc::new(Mutex::new(Pending::default()));
        let queue = Arc::clone(&pending);
        let foreground = thread::current();
        let control = OperationControl::with_observer(move |event| {
            queue
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(event);
            foreground.unpark();
        });
        let signal = control.clone();
        ctrlc::set_handler(move || signal.cancel()).map_err(io::Error::other)?;
        Ok(Self { pending, control })
    }

    fn run<T: Summary + Send>(
        self,
        stdout: &mut io::StdoutLock<'_>,
        json_stdout: bool,
        action: impl FnOnce(&OperationControl) -> Result<T, OperationError> + Send,
    ) -> io::Result<Result<T, OperationError>> {
        self.observe(
            stdout,
            json_stdout,
            action,
            |console, result| match result {
                Ok(report) => report.print(|line| console.line(line)),
                Err(error) => console.operation_failure(error),
            },
        )
    }

    fn observe<T: Send>(
        self,
        stdout: &mut io::StdoutLock<'_>,
        json_stdout: bool,
        action: impl FnOnce(&OperationControl) -> T + Send,
        finish: impl FnOnce(&mut Console<'_>, &T),
    ) -> io::Result<T> {
        let Self { pending, control } = self;
        let mut stderr = io::stderr();
        let terminal = if json_stdout {
            stderr.is_terminal()
        } else {
            stdout.is_terminal()
        } && std::env::var_os("TERM").is_none_or(|term| term != "dumb");
        let mut console = Console {
            out: if json_stdout { &mut stderr } else { stdout },
            error: None,
            partial: BTreeMap::new(),
            selection_shown: false,
            terminal,
            activity: None,
        };
        let pop = || {
            pending
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .pop()
        };
        let result = thread::scope(|scope| {
            let worker = scope.spawn(move || action(&control));
            while !worker.is_finished() {
                if let Some(event) = pop() {
                    console.event(event);
                } else {
                    thread::park_timeout(Duration::from_millis(25));
                    console.tick();
                }
            }
            let result = worker.join().unwrap();
            while let Some(event) = pop() {
                console.event(event);
            }
            result
        });
        console.clear_activity();
        finish(&mut console, &result);
        // A closed diagnostics stream must not prevent the JSON report from being written.
        if !json_stdout && let Some(error) = console.error {
            return Err(error);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_spinner_refreshes_and_clears_before_station_summary() {
        let mut buffer = Vec::new();
        {
            let mut console = Console {
                out: &mut buffer,
                error: None,
                partial: BTreeMap::new(),
                selection_shown: false,
                terminal: true,
                activity: None,
            };
            let date = chrono::NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
            console.download(DownloadEvent::AnaWindow {
                station: "123".into(),
                start: date,
                end: date,
                rows: 3,
                loading: false,
            });
            console.activity.as_mut().unwrap().drawn = Instant::now() - Duration::from_millis(100);
            console.tick();
            console.download(DownloadEvent::AnaFinished {
                station: "123".into(),
                dates: Some((date, date)),
                rows: 3,
                changed_samples: 2,
                error: None,
                cancelled: false,
            });
            assert!(console.activity.is_none());
        }
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("rows 3\n⠋ requesting"));
        assert!(output.contains("rows 3\n⠙ requesting"));
        assert!(output.contains("\r\x1b[2K\x1b[1A\r\x1b[2K"));
        assert!(output.ends_with("update 2026-07-20, 2026-07-20 | rows 3 | changed 2\n"));
    }
}
