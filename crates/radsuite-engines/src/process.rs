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

    let stdout = child.stdout.take().ok_or_else(|| ProcessError::Io {
        executable: executable.clone(),
        source: io::Error::new(io::ErrorKind::BrokenPipe, "stdout pipe was not created"),
    })?;
    let stderr = child.stderr.take().ok_or_else(|| ProcessError::Io {
        executable: executable.clone(),
        source: io::Error::new(io::ErrorKind::BrokenPipe, "stderr pipe was not created"),
    })?;

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

        if !cancelled && is_cancelled() {
            #[cfg(windows)]
            terminate_child(&mut child, &process_job, &executable)?;
            #[cfg(not(windows))]
            terminate_child(&mut child, &executable)?;
            cancelled = true;
        }

        if status.is_none() {
            status = child.try_wait().map_err(|source| ProcessError::Io {
                executable: executable.clone(),
                source,
            })?;
        }

        if status.is_none() || reader_count < 2 {
            thread::sleep(POLL_INTERVAL);
        }
    }

    stdout_thread.join().map_err(|_| ProcessError::Io {
        executable: executable.clone(),
        source: io::Error::other("stdout reader thread panicked"),
    })?;
    stderr_thread.join().map_err(|_| ProcessError::Io {
        executable: executable.clone(),
        source: io::Error::other("stderr reader thread panicked"),
    })?;

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

    if let Some(source) = reader_error {
        return Err(ProcessError::Io { executable, source });
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

fn kill_and_wait(child: &mut Child) -> io::Result<ExitStatus> {
    match child.kill() {
        Ok(()) => {}
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::InvalidInput | io::ErrorKind::NotFound
            ) => {}
        Err(error) => return Err(error),
    }
    child.wait()
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
fn terminate_child(child: &mut Child, executable: &Path) -> Result<(), ProcessError> {
    let pid = child.id() as libc::pid_t;
    let result = unsafe { libc::kill(-pid, libc::SIGKILL) };
    if result != 0 {
        let source = io::Error::last_os_error();
        if source.raw_os_error() == Some(libc::ESRCH) {
            return Ok(());
        }
        kill_and_wait(child).map_err(|fallback| ProcessError::Terminate {
            executable: executable.to_path_buf(),
            source: io::Error::new(
                fallback.kind(),
                format!(
                    "process group termination failed ({source}); child termination failed: {fallback}"
                ),
            ),
        })?;
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn terminate_child(child: &mut Child, executable: &Path) -> Result<(), ProcessError> {
    kill_and_wait(child)
        .map(|_| ())
        .map_err(|source| ProcessError::Terminate {
            executable: executable.to_path_buf(),
            source,
        })
}

#[cfg(windows)]
fn terminate_and_wait_after_job_assignment_failure(
    child: &mut Child,
    process_job: &WindowsProcessJob,
) -> io::Result<()> {
    let _ = process_job.terminate();
    kill_and_wait(child).map(|_| ())
}

#[cfg(windows)]
fn terminate_child(
    child: &mut Child,
    process_job: &WindowsProcessJob,
    executable: &Path,
) -> Result<(), ProcessError> {
    process_job
        .terminate()
        .or_else(|_| child.kill())
        .map_err(|source| ProcessError::Terminate {
            executable: executable.to_path_buf(),
            source,
        })
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
}
