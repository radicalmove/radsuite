use std::{
    ffi::OsString,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use thiserror::Error;

const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug)]
pub struct ProcessOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("could not start {executable}: {source}")]
    Start {
        executable: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed while running {executable}: {source}")]
    Io {
        executable: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{executable} was cancelled")]
    Cancelled { executable: PathBuf },
    #[error("could not terminate {executable}: {source}")]
    Terminate {
        executable: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{executable} failed: {message}")]
    Failed {
        executable: PathBuf,
        message: String,
    },
}

#[derive(Clone, Copy)]
enum OutputStream {
    Stdout,
    Stderr,
}

enum ReaderEvent {
    Chunk(OutputStream, Vec<u8>),
    Done,
    Error(String),
}

pub fn run_process<C, P>(
    executable: impl AsRef<Path>,
    args: &[OsString],
    mut is_cancelled: C,
    mut on_progress_line: P,
) -> Result<ProcessOutput, ProcessError>
where
    C: FnMut() -> bool,
    P: FnMut(&str),
{
    let executable = executable.as_ref().to_path_buf();
    let mut command = Command::new(&executable);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    configure_unix_process_group(&mut command);

    #[cfg(windows)]
    let process_job = WindowsProcessJob::new().map_err(|source| ProcessError::Start {
        executable: executable.clone(),
        source,
    })?;

    #[cfg(windows)]
    configure_windows_process_group(&mut command);

    let mut child = command.spawn().map_err(|source| ProcessError::Start {
        executable: executable.clone(),
        source,
    })?;

    #[cfg(windows)]
    if let Err(assignment_error) = process_job.assign(&child) {
        let source = match terminate_and_wait_after_job_assignment_failure(&mut child, &process_job)
        {
            Ok(()) => assignment_error,
            Err(cleanup_error) => io::Error::new(
                cleanup_error.kind(),
                format!(
                    "job assignment failed: {assignment_error}; child cleanup failed: {cleanup_error}"
                ),
            ),
        };
        return Err(ProcessError::Start { executable, source });
    }

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let cleanup_diagnostics = cleanup_spawned_child(
                &mut child,
                #[cfg(windows)]
                &process_job,
            );
            return Err(ProcessError::Io {
                executable,
                source: io::Error::other(combine_setup_diagnostics(
                    "stdout pipe was not created",
                    &cleanup_diagnostics,
                )),
            });
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            let cleanup_diagnostics = cleanup_spawned_child(
                &mut child,
                #[cfg(windows)]
                &process_job,
            );
            return Err(ProcessError::Io {
                executable,
                source: io::Error::other(combine_setup_diagnostics(
                    "stderr pipe was not created",
                    &cleanup_diagnostics,
                )),
            });
        }
    };

    let (sender, receiver) = mpsc::channel();
    let stdout_thread = spawn_reader(stdout, OutputStream::Stdout, sender.clone());
    let stderr_thread = spawn_reader(stderr, OutputStream::Stderr, sender);

    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    let mut stdout_pending = Vec::new();
    let mut reader_count = 0;
    let mut status = None;
    let mut cancelled = false;
    let mut reader_error = None;
    let mut termination_diagnostics = Vec::new();

    while status.is_none() || reader_count < 2 {
        drain_events(
            &receiver,
            &mut stdout_bytes,
            &mut stderr_bytes,
            &mut stdout_pending,
            &mut reader_count,
            &mut reader_error,
            &mut on_progress_line,
        );

        if status.is_none() && !cancelled && is_cancelled() {
            #[cfg(windows)]
            {
                status = terminate_child(&mut child, &process_job, &mut termination_diagnostics);
            }
            #[cfg(not(windows))]
            {
                status = terminate_child(&mut child, &mut termination_diagnostics);
            }
            cancelled = true;
        }

        if status.is_none() {
            match child.try_wait() {
                Ok(Some(next_status)) => status = Some(next_status),
                Ok(None) if cancelled => {
                    #[cfg(windows)]
                    {
                        status =
                            terminate_child(&mut child, &process_job, &mut termination_diagnostics);
                    }
                    #[cfg(not(windows))]
                    {
                        status = terminate_child(&mut child, &mut termination_diagnostics);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    termination_diagnostics.push(format!("child status polling failed: {error}"));
                    #[cfg(windows)]
                    {
                        status =
                            terminate_child(&mut child, &process_job, &mut termination_diagnostics);
                    }
                    #[cfg(not(windows))]
                    {
                        status = terminate_child(&mut child, &mut termination_diagnostics);
                    }
                    cancelled = true;
                }
            }
        }

        if status.is_none() || reader_count < 2 {
            thread::sleep(POLL_INTERVAL);
        }
    }

    let mut output_diagnostics = Vec::new();
    if stdout_thread.join().is_err() {
        output_diagnostics.push("stdout reader thread panicked".to_string());
    }
    if stderr_thread.join().is_err() {
        output_diagnostics.push("stderr reader thread panicked".to_string());
    }

    drain_events(
        &receiver,
        &mut stdout_bytes,
        &mut stderr_bytes,
        &mut stdout_pending,
        &mut reader_count,
        &mut reader_error,
        &mut on_progress_line,
    );
    if !stdout_pending.is_empty() {
        on_progress_line(&String::from_utf8_lossy(&stdout_pending));
    }

    if let Some(error) = reader_error {
        output_diagnostics.push(format!("output reader failed: {error}"));
    }
    if !termination_diagnostics.is_empty() {
        termination_diagnostics.extend(output_diagnostics);
        return Err(ProcessError::Terminate {
            executable,
            source: io::Error::other(combine_diagnostics(&termination_diagnostics)),
        });
    }
    if !output_diagnostics.is_empty() {
        return Err(ProcessError::Io {
            executable,
            source: io::Error::other(combine_diagnostics(&output_diagnostics)),
        });
    }
    if cancelled {
        return Err(ProcessError::Cancelled { executable });
    }

    let status = status.ok_or_else(|| ProcessError::Io {
        executable: executable.clone(),
        source: io::Error::other("child exited without an exit status"),
    })?;
    if !status.success() {
        return Err(ProcessError::Failed {
            executable,
            message: command_output(&stdout_bytes, &stderr_bytes),
        });
    }

    Ok(ProcessOutput {
        status,
        stdout: stdout_bytes,
        stderr: stderr_bytes,
    })
}

fn spawn_reader<R>(
    mut reader: R,
    stream: OutputStream,
    sender: Sender<ReaderEvent>,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    let _ = sender.send(ReaderEvent::Done);
                    return;
                }
                Ok(length) => {
                    if sender
                        .send(ReaderEvent::Chunk(stream, buffer[..length].to_vec()))
                        .is_err()
                    {
                        return;
                    }
                }
                Err(error) => {
                    let _ = sender.send(ReaderEvent::Error(error.to_string()));
                    let _ = sender.send(ReaderEvent::Done);
                    return;
                }
            }
        }
    })
}

fn drain_events<P>(
    receiver: &Receiver<ReaderEvent>,
    stdout_bytes: &mut Vec<u8>,
    stderr_bytes: &mut Vec<u8>,
    stdout_pending: &mut Vec<u8>,
    reader_count: &mut usize,
    reader_error: &mut Option<io::Error>,
    on_progress_line: &mut P,
) where
    P: FnMut(&str),
{
    while let Ok(event) = receiver.try_recv() {
        match event {
            ReaderEvent::Chunk(OutputStream::Stdout, bytes) => {
                stdout_bytes.extend_from_slice(&bytes);
                stdout_pending.extend_from_slice(&bytes);
                emit_complete_lines(stdout_pending, on_progress_line);
            }
            ReaderEvent::Chunk(OutputStream::Stderr, bytes) => {
                stderr_bytes.extend_from_slice(&bytes)
            }
            ReaderEvent::Done => *reader_count += 1,
            ReaderEvent::Error(message) => {
                if reader_error.is_none() {
                    *reader_error = Some(io::Error::other(message));
                }
            }
        }
    }
}

fn emit_complete_lines<P>(pending: &mut Vec<u8>, on_progress_line: &mut P)
where
    P: FnMut(&str),
{
    while let Some(index) = pending.iter().position(|byte| *byte == b'\n') {
        let mut line = pending.drain(..=index).collect::<Vec<_>>();
        line.pop();
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        on_progress_line(&String::from_utf8_lossy(&line));
    }
}

fn command_output(stdout: &[u8], stderr: &[u8]) -> String {
    let output = if stderr.is_empty() { stdout } else { stderr };
    let message = String::from_utf8_lossy(output).trim().to_string();
    if message.is_empty() {
        "no diagnostic output".to_string()
    } else {
        message
    }
}

fn combine_diagnostics(diagnostics: &[String]) -> String {
    diagnostics.join("; ")
}

fn combine_setup_diagnostics(message: &str, cleanup_diagnostics: &[String]) -> String {
    if cleanup_diagnostics.is_empty() {
        message.to_string()
    } else {
        format!(
            "{message}; child cleanup failed: {}",
            combine_diagnostics(cleanup_diagnostics)
        )
    }
}

fn kill_and_wait(child: &mut Child) -> io::Result<ExitStatus> {
    let kill_error = match child.kill() {
        Ok(()) => None,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::InvalidInput | io::ErrorKind::NotFound
            ) =>
        {
            None
        }
        Err(error) => Some(error),
    };
    let wait_result = child.wait();
    match (kill_error, wait_result) {
        (None, Ok(status)) => Ok(status),
        (Some(kill_error), Ok(_)) => Err(io::Error::new(
            kill_error.kind(),
            format!("child kill failed but child was reaped: {kill_error}"),
        )),
        (None, Err(wait_error)) => Err(wait_error),
        (Some(kill_error), Err(wait_error)) => Err(io::Error::new(
            wait_error.kind(),
            format!("child kill failed: {kill_error}; child wait failed: {wait_error}"),
        )),
    }
}

#[cfg(unix)]
fn cleanup_spawned_child(child: &mut Child) -> Vec<String> {
    let mut diagnostics = Vec::new();
    let _ = terminate_child(child, &mut diagnostics);
    diagnostics
}

#[cfg(windows)]
fn cleanup_spawned_child(child: &mut Child, process_job: &WindowsProcessJob) -> Vec<String> {
    let mut diagnostics = Vec::new();
    let _ = terminate_child(child, process_job, &mut diagnostics);
    diagnostics
}

#[cfg(not(any(unix, windows)))]
fn cleanup_spawned_child(child: &mut Child) -> Vec<String> {
    let mut diagnostics = Vec::new();
    let _ = terminate_child(child, &mut diagnostics);
    diagnostics
}

#[cfg(unix)]
fn configure_unix_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(windows)]
fn configure_windows_process_group(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP);
}

#[cfg(unix)]
fn terminate_child(child: &mut Child, diagnostics: &mut Vec<String>) -> Option<ExitStatus> {
    let pid = child.id() as libc::pid_t;
    let result = unsafe { libc::kill(-pid, libc::SIGKILL) };
    if result != 0 {
        let source = io::Error::last_os_error();
        if source.raw_os_error() != Some(libc::ESRCH) {
            diagnostics.push(format!("Unix process-group termination failed: {source}"));
        }
    }
    match kill_and_wait(child) {
        Ok(status) => Some(status),
        Err(error) => {
            diagnostics.push(format!("Unix child kill/wait failed: {error}"));
            None
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn terminate_child(child: &mut Child, diagnostics: &mut Vec<String>) -> Option<ExitStatus> {
    match kill_and_wait(child) {
        Ok(status) => Some(status),
        Err(error) => {
            diagnostics.push(format!("child kill/wait failed: {error}"));
            None
        }
    }
}

#[cfg(windows)]
fn terminate_and_wait_after_job_assignment_failure(
    child: &mut Child,
    process_job: &WindowsProcessJob,
) -> io::Result<()> {
    let mut diagnostics = Vec::new();
    if let Err(error) = process_job.terminate() {
        diagnostics.push(format!("Windows Job Object termination failed: {error}"));
    }
    if let Err(error) = terminate_windows_process_tree(child.id()) {
        diagnostics.push(format!("taskkill process-tree termination failed: {error}"));
    }
    if let Err(error) = kill_and_wait(child) {
        diagnostics.push(format!("Windows child kill/wait failed: {error}"));
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(combine_diagnostics(&diagnostics)))
    }
}

#[cfg(windows)]
fn terminate_child(
    child: &mut Child,
    process_job: &WindowsProcessJob,
    diagnostics: &mut Vec<String>,
) -> Option<ExitStatus> {
    if let Err(error) = process_job.terminate() {
        diagnostics.push(format!("Windows Job Object termination failed: {error}"));
        if let Err(error) = terminate_windows_process_tree(child.id()) {
            diagnostics.push(format!("taskkill process-tree termination failed: {error}"));
        }
    }
    match kill_and_wait(child) {
        Ok(status) => Some(status),
        Err(error) => {
            diagnostics.push(format!("Windows child kill/wait failed: {error}"));
            None
        }
    }
}

#[cfg(windows)]
fn terminate_windows_process_tree(pid: u32) -> io::Result<()> {
    let pid = pid.to_string();
    let output = Command::new("taskkill")
        .args(["/PID", pid.as_str(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "taskkill exited with {}: {}",
            output.status,
            command_output(&output.stdout, &output.stderr)
        )))
    }
}

#[cfg(windows)]
struct WindowsProcessJob {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl WindowsProcessJob {
    fn new() -> io::Result<Self> {
        use std::{mem::size_of, ptr::null};
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };

        let handle = unsafe { CreateJobObjectW(null(), null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }

        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let result = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if result == 0 {
            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
            return Err(io::Error::last_os_error());
        }
        Ok(Self { handle })
    }

    fn assign(&self, child: &Child) -> io::Result<()> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;

        let result = unsafe {
            AssignProcessToJobObject(
                self.handle,
                child.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE,
            )
        };
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn terminate(&self) -> io::Result<()> {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;

        let result = unsafe { TerminateJobObject(self.handle, 1) };
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(windows)]
impl Drop for WindowsProcessJob {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn kill_and_wait_helper_reaps_a_running_child() {
        let started = std::time::Instant::now();
        let mut child = Command::new("sh")
            .args(["-c", "sleep 30"])
            .spawn()
            .expect("spawn test child");

        let status = kill_and_wait(&mut child).expect("kill and wait for test child");

        assert!(!status.success());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn diagnostics_preserve_every_cleanup_failure() {
        let diagnostics = vec![
            "terminate failed".to_string(),
            "kill failed".to_string(),
            "wait failed".to_string(),
        ];

        assert_eq!(
            combine_setup_diagnostics("child setup failed", &diagnostics),
            "child setup failed; child cleanup failed: terminate failed; kill failed; wait failed"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_process_tree_fallback_uses_numeric_pid_termination() {
        let mut child = Command::new("cmd")
            .args(["/C", "ping 127.0.0.1 -n 30 >NUL"])
            .spawn()
            .expect("spawn Windows test child");
        let started = std::time::Instant::now();

        terminate_windows_process_tree(child.id()).expect("terminate Windows process tree");
        let _ = child.wait();

        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
