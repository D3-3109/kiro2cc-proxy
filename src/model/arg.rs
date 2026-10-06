// Copyright (c) 2026 Harllan He. Licensed under MIT.

//! 命令行参数定义（clap::Parser）：--config / --credentials 等启动项。
use clap::Parser;

/// Anthropic <-> Kiro API 客户端
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// 配置文件路径
    #[arg(short, long)]
    pub config: Option<String>,

    /// 凭证文件路径
    #[arg(long)]
    pub credentials: Option<String>,
}
