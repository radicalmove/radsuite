use std::io;

use tokio::process::{Child, Command};

#[derive(Debug)]
pub struct ManagedProcessGroup {
    #[cfg(windows)]
    job: windows_job::Job,
}

pub type ManagedChild = (Child, ManagedProcessGroup);

impl ManagedProcessGroup {
    pub fn attach(command: &mut Command) -> io::Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.as_std_mut().process_group(0);
            Ok(Self {})
        }

        #[cfg(windows)]
        {
            let job = windows_job::Job::new()?;
            job.configure_kill_on_close()?;
            Ok(Self { job })
        }
    }

    pub fn register_child(&self, child: &Child) -> io::Result<()> {
        #[cfg(unix)]
        {
            let _ = child;
            Ok(())
        }

        #[cfg(windows)]
        {
            let handle = child.raw_handle().ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "child process handle is unavailable")
            })?;
            self.job.assign_process(handle)
        }
    }

    pub fn terminate(&self, child: &Child, force: bool) -> io::Result<()> {
        #[cfg(unix)]
        {
            if let Some(pid) = child.id() {
                let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
                unsafe {
                    if libc::kill(-(pid as libc::pid_t), signal) != 0 {
                        return Err(io::Error::last_os_error());
                    }
                }
            }
            Ok(())
        }

        #[cfg(windows)]
        {
            let _ = child;
            let _ = force;
            self.job.terminate()
        }
    }
}

#[cfg(windows)]
mod windows_job {
    use std::{io, mem, os::windows::io::RawHandle, ptr};

    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, HANDLE},
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject,
        },
    };

    #[derive(Debug)]
    pub struct Job(HANDLE);

    impl Job {
        pub fn new() -> io::Result<Self> {
            let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
            if handle == 0 {
                return Err(last_error());
            }
            Ok(Self(handle))
        }

        pub fn configure_kill_on_close(&self) -> io::Result<()> {
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { mem::zeroed() };
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let result = unsafe {
                SetInformationJobObject(
                    self.0,
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as *const _,
                    mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            if result == 0 {
                return Err(last_error());
            }
            Ok(())
        }

        pub fn assign_process(&self, raw_handle: RawHandle) -> io::Result<()> {
            let result = unsafe { AssignProcessToJobObject(self.0, raw_handle as HANDLE) };
            if result == 0 {
                return Err(last_error());
            }
            Ok(())
        }

        pub fn terminate(&self) -> io::Result<()> {
            let result = unsafe { TerminateJobObject(self.0, 1) };
            if result == 0 {
                return Err(last_error());
            }
            Ok(())
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    fn last_error() -> io::Error {
        io::Error::from_raw_os_error(unsafe { GetLastError() } as i32)
    }
}
