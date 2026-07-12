pub mod net;
pub mod secure;
pub mod ssh;
pub mod sync;
pub mod system;
pub mod user;

use crate::{RemoteHost, prelude::*};

pub struct SshConnection {
    pub target: String,
    pub args: Vec<String>,
}

pub fn get_remote_host_ext(target: &Option<String>, ip: &Option<String>) -> Result<RemoteHost> {
    let cfg = Settings::get();

    // 1. Specific IP [--ip]
    if let Some(explicit_ip) = ip {
        // search in config similarity ip
        if let Some(found_host) = cfg
            .remote
            .hosts
            .values()
            .find(|h| &h.ip_addr == explicit_ip)
        {
            return Ok(found_host.clone());
        }

        // or use default options
        return Ok(RemoteHost {
            ip_addr: explicit_ip.clone(),
            user_name: "root".to_string(),
            ssh_file: None,
        });
    }

    // 2. Search by target name
    let host_name = target.as_deref().unwrap_or(&cfg.remote.default);
    cfg.remote
        .hosts
        .get(host_name)
        .cloned()
        .ok_or_else(|| Error::UnknownHost(host_name.to_string()).into())
}

pub fn get_ssh_conn(target: &Option<String>, ip: &Option<String>) -> Result<SshConnection> {
    let host = get_remote_host_ext(target, ip)?;
    let target_str = format!("{}@{}", host.user_name, host.ip_addr);
    let mut args = Vec::new();

    if let Some(ref key_path) = host.ssh_file {
        args.push("-i".to_string());
        args.push(key_path.to_string_lossy().into_owned());
    }

    Ok(SshConnection {
        target: target_str,
        args,
    })
}

pub fn get_remote_host(
    target: &Option<String>,
    explicit_ip: &Option<String>,
) -> Result<RemoteHost> {
    let cfg = Settings::get();

    // 1. Если передан явный IP через --ip
    if let Some(ip) = explicit_ip {
        // Ищем в конфиге хост, у которого такой же IP (чтобы подтянуть его юзера и ключ, если он там есть)
        if let Some(found_host) = cfg.remote.hosts.values().find(|h| &h.ip_addr == ip) {
            return Ok(found_host.clone());
        }

        // Если с таким IP в конфиге ничего нет, берем дефолтного хоста как шаблон
        // (чтобы скопировать дефолтного user_name и ssh_file), но меняем IP на указанный
        if let Some(default_host) = cfg.remote.hosts.get(&cfg.remote.default) {
            let mut host = default_host.clone();
            host.ip_addr = ip.clone(); // Подменяем IP на лету
            return Ok(host);
        }

        // Если даже дефолтного хоста нет в конфиге, собираем минимальный RemoteHost
        // (подставьте ваши реальные поля структуры RemoteHost)
        return Ok(RemoteHost {
            ip_addr: ip.clone(),
            user_name: "root".to_string(), // или другое значение по умолчанию
            ssh_file: None,
            // другие поля, если они есть...
        });
    }

    // 2. Если --ip не указан, работаем по старой логике (ищем по имени хоста)
    let host_name = target.as_deref().unwrap_or(&cfg.remote.default);

    cfg.remote
        .hosts
        .get(host_name)
        .cloned()
        .ok_or_else(|| Error::UnknownHost(host_name.to_string()).into())
}

pub fn log() -> ColoredString {
    "==>".blue()
}
pub fn ok() -> ColoredString {
    " ->".green()
}
pub fn err() -> ColoredString {
    " ->".red()
}
pub fn warn() -> ColoredString {
    " ->".yellow()
}
