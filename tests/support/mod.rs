use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static SERVER_LOCK: Mutex<()> = Mutex::new(());
const ADDRESS: &str = "127.0.0.1:7379";

#[derive(Debug, PartialEq)]
pub enum Reply {
    Simple(Vec<u8>),
    Error(Vec<u8>),
    Integer(i64),
    Bulk(Option<Vec<u8>>),
}

pub fn bulk(value: &str) -> Reply {
    Reply::Bulk(Some(value.as_bytes().to_vec()))
}

pub fn encode(args: &[&[u8]]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", args.len()).into_bytes();
    for arg in args {
        write!(out, "${}\r\n", arg.len()).unwrap();
        out.extend_from_slice(arg);
        out.extend_from_slice(b"\r\n");
    }
    out
}

pub struct Client {
    pub stream: TcpStream,
    reader: BufReader<TcpStream>,
}

impl Client {
    pub fn connect() -> Self {
        let stream = TcpStream::connect_timeout(
            &ADDRESS.parse::<SocketAddr>().unwrap(),
            Duration::from_secs(2),
        )
        .expect("connect to test server");
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        Self {
            reader: BufReader::new(stream.try_clone().unwrap()),
            stream,
        }
    }

    pub fn read(&mut self) -> std::io::Result<Reply> {
        let mut line = Vec::new();
        if self.reader.read_until(b'\n', &mut line)? == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "server closed connection",
            ));
        }
        if !line.ends_with(b"\r\n") || line.len() < 3 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid RESP line: {line:?}"),
            ));
        }
        let payload = &line[1..line.len() - 2];
        match line[0] {
            b'+' => Ok(Reply::Simple(payload.to_vec())),
            b'-' => Ok(Reply::Error(payload.to_vec())),
            b':' | b'$' => {
                let number = std::str::from_utf8(payload)
                    .ok()
                    .and_then(|s| s.parse::<i64>().ok())
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid RESP number")
                    })?;
                if line[0] == b':' {
                    return Ok(Reply::Integer(number));
                }
                if number == -1 {
                    return Ok(Reply::Bulk(None));
                }
                let size = usize::try_from(number)
                    .ok()
                    .filter(|size| *size <= 32 * 1024 * 1024)
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid bulk length")
                    })?;
                let mut data = vec![0; size];
                self.reader.read_exact(&mut data)?;
                let mut terminator = [0; 2];
                self.reader.read_exact(&mut terminator)?;
                if terminator != *b"\r\n" {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invalid bulk terminator",
                    ));
                }
                Ok(Reply::Bulk(Some(data)))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid RESP prefix: {line:?}"),
            )),
        }
    }

    pub fn bytes(&mut self, args: &[&[u8]]) -> Reply {
        self.stream.write_all(&encode(args)).unwrap();
        self.read().expect("complete RESP reply")
    }

    pub fn cmd(&mut self, args: &[&str]) -> Reply {
        self.bytes(&args.iter().map(|s| s.as_bytes()).collect::<Vec<_>>())
    }

    pub fn pipeline(&mut self, commands: &[Vec<String>]) -> Vec<Reply> {
        let mut wire = Vec::new();
        for args in commands {
            wire.extend(encode(
                &args.iter().map(|s| s.as_bytes()).collect::<Vec<_>>(),
            ));
        }
        self.stream.write_all(&wire).unwrap();
        (0..commands.len())
            .map(|_| self.read().expect("complete pipelined reply"))
            .collect()
    }
}

pub struct Server {
    child: Option<Child>,
    pub directory: PathBuf,
    temp_root: PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl Server {
    pub fn new() -> Self {
        // The server currently has a fixed port, so fixtures run serially.
        let lock = SERVER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let probe =
            TcpListener::bind(ADDRESS).expect("port 7379 must be free; stop any existing server");
        drop(probe);
        let temp_root = std::env::temp_dir().canonicalize().unwrap();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = temp_root.join(format!("redis-rust-audit-{}-{nonce}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let mut server = Self {
            child: None,
            directory,
            temp_root,
            _lock: lock,
        };
        server.start();
        server
    }

    pub fn start(&mut self) {
        let stderr = File::options()
            .create(true)
            .append(true)
            .open(self.directory.join("stderr.log"))
            .unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_vynk"));
        command
            .current_dir(&self.directory)
            .stdout(Stdio::null())
            .stderr(stderr);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        self.child = Some(command.spawn().expect("start isolated server"));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                panic!("server exited during startup: {status}");
            }
            if TcpStream::connect_timeout(&ADDRESS.parse().unwrap(), Duration::from_millis(50))
                .is_ok()
            {
                break;
            }
            assert!(Instant::now() < deadline, "server did not become ready");
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if child.try_wait().unwrap().is_none() {
                child.kill().unwrap();
            }
            child.wait().unwrap();
        }
    }

    pub fn restart(&mut self) {
        self.stop();
        self.start();
    }

    pub fn assert_alive(&mut self) {
        assert!(
            self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
            "server crashed"
        );
        assert_eq!(
            Client::connect().cmd(&["PING"]),
            Reply::Simple(b"PONG".to_vec())
        );
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
        if std::thread::panicking()
            && let Ok(log) = fs::read_to_string(self.directory.join("stderr.log"))
        {
            eprintln!("server stderr:\n{log}");
        }
        if let Ok(target) = self.directory.canonicalize()
            && target.parent() == Some(self.temp_root.as_path())
        {
            let _ = fs::remove_dir_all(target);
        }
    }
}
