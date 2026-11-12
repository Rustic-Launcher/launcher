//! The companion's network: one worker thread of its own (never the render thread, never the
//! game's tick), fed by a channel; results are collected and applied on the next client tick.
//! The session token lives only in this thread's memory.
use rusticmc_companion_core::protocol::{
    Endpoints, LookupAnswer, Method, Request, Session, SocialSnapshot, WornAnswer, WornItem,
};
use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// At most this much is read from any answer (a bundle is at most 256 KiB).
const MAX_BODY: u64 = 512 * 1024;

pub enum Job {
    /// Trade the launcher's one-time companion token for a session.
    Session(String),
    Worn(Vec<u128>),
    /// Who among these players uses Rustic (the badge).
    Badges(Vec<u128>),
    Bundle(WornItem),
    Social,
    Invite {
        to: u128,
        address: String,
    },
    Accept(String),
    Playing(Option<String>),
}

pub enum Done {
    Session(Result<Session, String>),
    Worn(Result<WornAnswer, String>),
    /// The players of the batch who use Rustic.
    Badges(Result<Vec<u128>, String>),
    Bundle {
        hash: String,
        bytes: Result<Vec<u8>, String>,
    },
    Social(Result<SocialSnapshot, String>),
    Action {
        what: &'static str,
        result: Result<(), String>,
    },
}

pub struct Net {
    tx: Mutex<Sender<Job>>,
    done: Arc<Mutex<Vec<Done>>>,
}

impl Net {
    pub fn start(endpoints: Endpoints) -> Net {
        let (tx, rx) = channel();
        let done = Arc::new(Mutex::new(Vec::new()));
        let out = done.clone();
        let spawned = std::thread::Builder::new()
            .name("Rustic companion".into())
            .spawn(move || worker(endpoints, rx, out));
        if let Err(e) = spawned {
            log::error!("Rustic: the companion's network thread did not start: {e}");
        }
        Net {
            tx: Mutex::new(tx),
            done,
        }
    }

    pub fn send(&self, job: Job) {
        let tx = self.tx.lock().unwrap_or_else(|e| e.into_inner());
        let _ = tx.send(job);
    }

    pub fn drain(&self) -> Vec<Done> {
        std::mem::take(&mut *self.done.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

fn call(agent: &ureq::Agent, req: &Request, token: Option<&str>) -> Result<Vec<u8>, String> {
    let mut r = match req.method {
        Method::Get => agent.get(&req.url),
        Method::Post => agent.post(&req.url),
    };
    if let Some(b) = &req.bearer {
        r = r.set("Authorization", &format!("Bearer {b}"));
    } else if req.auth {
        match token {
            Some(t) => r = r.set("Authorization", &format!("Bearer {t}")),
            None => return Err("not signed in".into()),
        }
    }
    let resp = match &req.body {
        Some(body) => r.set("Content-Type", "application/json").send_string(body),
        None => r.call(),
    };
    match resp {
        Ok(resp) => {
            let mut bytes = Vec::new();
            resp.into_reader()
                .take(MAX_BODY + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_BODY {
                return Err("answer too large".into());
            }
            Ok(bytes)
        }
        Err(ureq::Error::Status(code, _)) => Err(format!("HTTP {code}")),
        Err(e) => Err(e.to_string()),
    }
}

fn json<T: serde::de::DeserializeOwned>(r: Result<Vec<u8>, String>) -> Result<T, String> {
    serde_json::from_slice(&r?).map_err(|e| e.to_string())
}

fn worker(e: Endpoints, rx: Receiver<Job>, out: Arc<Mutex<Vec<Done>>>) {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("rustic-companion/", env!("CARGO_PKG_VERSION")))
        .build();
    let mut token: Option<String> = None;
    let push = |d: Done| out.lock().unwrap_or_else(|e| e.into_inner()).push(d);
    // A panic here (a bug) ends the thread, never the game.
    for job in rx {
        let t = token.as_deref();
        match job {
            Job::Session(companion_token) => {
                let r: Result<Session, String> = match e.session(&companion_token) {
                    Some(req) => json(call(&agent, &req, None)),
                    None => Err("no loopback endpoint from the launcher".into()),
                };
                if let Ok(s) = &r {
                    token = Some(s.token.clone());
                }
                push(Done::Session(r));
            }
            Job::Worn(players) => push(Done::Worn(json(call(&agent, &e.worn(&players), t)))),
            Job::Badges(players) => {
                let r: Result<LookupAnswer, String> =
                    json(call(&agent, &e.rustic_lookup(&players), None));
                push(Done::Badges(r.map(|a| a.rustic())));
            }
            Job::Bundle(item) => {
                let bytes = match e.bundle(&item) {
                    Some(req) => call(&agent, &req, None),
                    None => Err("bad bundle address".into()),
                };
                push(Done::Bundle {
                    hash: item.hash,
                    bytes,
                });
            }
            Job::Social => push(Done::Social(json(call(&agent, &e.social(), t)))),
            Job::Invite { to, address } => push(Done::Action {
                what: "invite",
                result: call(&agent, &e.invite(to, &address), t).map(|_| ()),
            }),
            Job::Accept(id) => push(Done::Action {
                what: "accept",
                result: match e.accept(&id) {
                    Some(req) => call(&agent, &req, t).map(|_| ()),
                    None => Err("bad invite id".into()),
                },
            }),
            Job::Playing(emote) => {
                let _ = call(&agent, &e.playing(emote.as_deref()), t);
            }
        }
    }
}
