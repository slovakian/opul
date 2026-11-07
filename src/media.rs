use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::collections::{HashMap, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use url::Url;
use uuid::Uuid;

use crate::CoreError;
use crate::destination::{DownloadContentLayout, resolve_download_destination};
use crate::persistence::{LoadedState, load_state, save_state};
use crate::video_tools::{VideoTools, VideoToolsInvocation};

pub use probe::classify_guided_url;

mod types;
pub use types::*;

mod guided;
use guided::*;

mod guided_rules;
use guided_rules::*;

mod open;
use open::*;

mod queue;

mod control;

mod context;

mod probe;
use probe::*;

mod workers;
use workers::*;

mod direct;
use direct::*;

mod destinations;
use destinations::*;

mod progress;
use progress::*;

#[cfg(test)]
mod tests;
