// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

/// Returns the new limit.
pub fn set_open_file_limit(limit: u64) -> Result<u64, String> {
    #[cfg(unix)]
    return {
        use nix::sys::resource::{getrlimit, setrlimit, Resource};
        let (_, hard) = getrlimit(Resource::RLIMIT_NOFILE).map_err(|e| e.to_string())?;
        let new = limit.min(hard);
        setrlimit(Resource::RLIMIT_NOFILE, new, hard).map_err(|e| e.to_string())?;
        Ok(new)
    };

    #[cfg(not(unix))]
    Err(String::from("unsupported OS"))
}
