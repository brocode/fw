use crate::errors::AppError;
use dirs::config_dir;
use std::env;
use std::path::PathBuf;

pub struct FwPaths {
    pub settings: PathBuf,
    pub base: PathBuf,
    pub projects: PathBuf,
    pub tags: PathBuf,
}

impl FwPaths {
    pub fn ensure_base_exists(&self) -> Result<(), AppError> {
        std::fs::create_dir_all(&self.base).map_err(|e| AppError::RuntimeError(format!("Failed to create fw config base directory. {}", e)))?;
        Ok(())
    }
}

fn do_expand(path: PathBuf, home_dir: Option<PathBuf>) -> PathBuf {
    if let Some(home) = home_dir {
        home.join(path.strip_prefix("~").expect("only doing this if path starts with ~"))
    } else {
        path
    }
}

pub fn expand_path(path: PathBuf) -> PathBuf {
    if path.starts_with("~") {
        do_expand(path, dirs::home_dir())
    } else {
        path
    }
}

pub fn fw_path() -> Result<FwPaths, AppError> {
    let base = env::var("FW_CONFIG_DIR")
        .map(PathBuf::from)
        .ok()
        .map(expand_path)
        .or_else(|| {
            config_dir().map(|mut c| {
                c.push("fw");
                c
            })
        })
        .ok_or(AppError::InternalError("Cannot resolve fw config dir"))?;

    let mut settings = base.clone();

    let env: String = env::var_os("FW_ENV")
        .map(|s| s.to_string_lossy().to_string())
        .map(|s| format!("{}_", s))
        .unwrap_or_default()
        .replace('/', "");

    settings.push(format!("{}settings.toml", env));

    let mut projects = base.clone();
    projects.push("projects");

    let mut tags = base.clone();
    tags.push("tags");

    Ok(FwPaths {
        settings,
        base,
        projects,
        tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_do_not_expand_path_without_tilde() {
        let path = PathBuf::from("/foo/bar");
        assert_eq!(expand_path(path.clone()), path);
    }
    #[test]
    fn test_do_expand_path() {
        let path = PathBuf::from("~/foo/bar");
        let home = PathBuf::from("/my/home");
        assert_eq!(do_expand(path, Some(home)), PathBuf::from("/my/home/foo/bar"));
    }
}
