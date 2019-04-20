use super::TimeSpec;
use mio::{unix::EventedFd, Poll, PollOpt, Ready, Token};
use nix::sys::event::*;
use std::io;
use std::os::unix::io::RawFd;

pub(crate) struct Timer(RawFd);

impl Timer {
    pub(crate) fn new() -> io::Result<Self> {
        let kq = kqueue().map_err(|e| e.as_errno().unwrap())?;
        Ok(Timer(kq))
    }

    pub(crate) fn set(&mut self, timer: TimeSpec) -> io::Result<()> {
        let mut flags = EV_ADD | EV_ENABLE;
        if let TimeSpec::Timeout(..) = timer {
            flags |= EV_ONESHOT;
        }
        let time = match timer {
            TimeSpec::Delay(d) | TimeSpec::Interval(d) => d.as_secs() * 1_000 + d.subsec_millis(),
        };

        kevent(
            self.0,
            &[KEvent {
                ident: 1,
                filter: EventFilter::EVFILT_TIMER,
                flags,
                fflags: FilterFlag::empty(),
                data: time,
                udata: 0,
            }],
            &mut [],
            0,
        )
        .map_err(|e| e.as_errno().unwrap())?;

        Ok(())
    }

    pub(crate) fn check(&mut self) -> io::Result<()> {
        let mut ev = [KEvent::default()];
        match kevent(self.0, &[], &mut ev[..], 0).map_err(|e| e.as_errno().unwrap())? {
            1 => {
                // timer fired!
                assert_eq!(ev[0].ident, 1);
                Ok(())
            }
            0 => {
                // timer has not fired?
                Err(io::Error::new(io::ErrorKind::WouldBlod))
            }
            n => unreachable!(),
        }
    }
}

impl mio::Evented for Timer {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.0).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.0).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        EventedFd(&self.0).deregister(poll)
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let _ = nix::unistd::close(self.0);
    }
}
