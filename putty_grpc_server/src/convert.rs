//! Bidirectional conversion helpers between the protobuf world
//! and the domain structs that live in putty_core.

use putty_core::Profile;
use tonic::Status;

use crate::putty_interface::{profile_req, ProfileReq, Serial, Ssh};

/// core ▸ protobuf
impl From<Profile> for ProfileReq {
    fn from(p: Profile) -> Self {
        match p {
            Profile::Serial { name, port, baud } => ProfileReq {
                name,
                kind: Some(profile_req::Kind::Serial(Serial { port, baud })),
            },
            Profile::Ssh {
                name,
                host,
                port,
                username,
                password,
            } => ProfileReq {
                name,
                kind: Some(profile_req::Kind::Ssh(Ssh {
                    host,
                    port: port as u32,
                    user: username,
                    password,
                })),
            },
        }
    }
}

/// protobuf ▸ core
impl TryFrom<ProfileReq> for Profile {
    type Error = Status; // so `?` works inside tonic handlers

    fn try_from(m: ProfileReq) -> Result<Self, Self::Error> {
        let kind = m
            .kind
            .ok_or_else(|| Status::invalid_argument("Profile.kind missing"))?;
        match kind {
            profile_req::Kind::Serial(s) => Ok(Profile::Serial {
                name: m.name,
                port: s.port,
                baud: s.baud,
            }),
            profile_req::Kind::Ssh(s) => Ok(Profile::Ssh {
                name: m.name,
                host: s.host,
                port: s.port as u16,
                username: s.user,
                password: s.password,
            }),
        }
    }
}
