use std::{net::SocketAddr, num::NonZeroUsize, time::Duration, usize};

use divan::Bencher;
use mio::{event::Source, Events};

const SAMPLE_SIZE: u32 = 1000;
const SAMPLE_COUNT: u32 = 1000;

#[divan::bench(sample_size = SAMPLE_SIZE, sample_count = SAMPLE_COUNT)]
fn _1_register_mio(bencher: Bencher) {
    use mio::{net::TcpListener, Interest, Poll, Token};
    use std::net::TcpStream;

    let mut poll = Poll::new().unwrap();
    let addr: SocketAddr = "[::]:0".parse().unwrap();
    let mut listener = TcpListener::bind(addr).unwrap();

    let registry = poll.registry().try_clone().unwrap();
    let addr = listener.local_addr().unwrap();
    let mut events = Events::with_capacity(1000);
    registry
        .register(&mut listener, Token(usize::MAX), Interest::READABLE)
        .unwrap();
    let _stream = TcpStream::connect(addr).unwrap();
    poll.poll(&mut events, None).unwrap();
    let (mut stream, _addr) = listener.accept().unwrap();
    bencher.bench_local(|| {
        stream
            .register(&registry, Token(0), Interest::READABLE)
            .unwrap();
        stream.deregister(&registry).unwrap();
    });
}

#[divan::bench(sample_size = SAMPLE_SIZE, sample_count = SAMPLE_COUNT)]
fn _1_register_polling(bencher: Bencher) {
    use polling::{Event, Poller};
    use std::net::{TcpListener, TcpStream};

    let addr: SocketAddr = "[::]:0".parse().unwrap();
    let listener = TcpListener::bind(addr).unwrap();
    let poller = Poller::new().unwrap();
    let addr = listener.local_addr().unwrap();
    let _stream = TcpStream::connect(addr).unwrap();
    let (stream, _addr) = listener.accept().unwrap();
    stream.set_nonblocking(true).unwrap();
    bencher.bench_local(|| {
        unsafe {
            poller
                .add_with_mode(&stream, Event::readable(0), polling::PollMode::EdgeOneshot)
                .unwrap()
        }
        poller.delete(&stream).unwrap();
    });
}

#[divan::bench(sample_size = SAMPLE_SIZE, sample_count = SAMPLE_COUNT)]
fn _2_poll_mio(bencher: Bencher) {
    use mio::{Events, Poll};
    use std::time::Duration;

    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1000);

    bencher.bench_local(|| {
        poll.poll(&mut events, Some(Duration::ZERO)).unwrap();
    });
}

#[divan::bench(sample_size = SAMPLE_SIZE, sample_count = SAMPLE_COUNT)]
fn _2_poll_polling(bencher: Bencher) {
    use polling::{Events, Poller};

    let poller = Poller::new().unwrap();
    let mut events = Events::with_capacity(unsafe { NonZeroUsize::new_unchecked(1000) });

    bencher.bench_local(|| {
        poller.wait(&mut events, Some(Duration::ZERO)).unwrap();
    })
}
fn main() {
    adjust_file_descriptor_limits();
    divan::main()
}

pub fn adjust_file_descriptor_limits() {
    let mut limits = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };

    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limits) } != 0 {
        panic!(
            "get file handle limit err: {}",
            std::io::Error::last_os_error()
        );
    };

    limits.rlim_cur = limits.rlim_max;

    if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limits) } != 0 {
        panic!(
            "set file handle limit err: {}",
            std::io::Error::last_os_error()
        );
    }
}
